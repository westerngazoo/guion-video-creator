//! R-0008: generic synthesis. One test per acceptance criterion, each
//! against a closed form or an exact count.

use std::f64::consts::{PI, TAU};

use guion_audio::synth::{
    self, breakpoints, duck_follower, duck_windows, exp_decay, fade, fft, filter, normalize_peak,
    pan_gains, partials, phase_of, sample_at, seed_for, sine, to_i16, to_pcm16, Adsr, Bus,
    DuckShape, Follower, Noise, OnePole, Pan, Partial, Room, Stereo,
};
use guion_audio::{read_wav_pcm16, write_wav, AudioError};

const SR: f64 = 48_000.0;

/// A small scene through every stage: notes, noise, a room, a duck, a WAV.
fn scene(seed: u64, extra_event: bool) -> (Vec<u8>, Vec<f64>) {
    let n = (2.0 * SR) as usize;
    let mut bus = Bus::new(n);
    let mut hits = Vec::new();
    let events: &[f64] = if extra_event {
        &[0.1, 0.5, 0.9]
    } else {
        &[0.1, 0.9]
    };
    for (i, &t) in events.iter().enumerate() {
        // the event's noise depends on its own index only
        let idx = if extra_event && i == 1 {
            99
        } else if i == 0 {
            0
        } else {
            1
        };
        let mut g = Noise::new(seed_for(seed, idx));
        let m = (0.3 * SR) as usize;
        let hit: Vec<f64> = g
            .white(m)
            .iter()
            .zip(exp_decay(m, SR, 0.05))
            .map(|(a, b)| a * b)
            .collect();
        if idx != 99 {
            hits.push(hit.clone());
        }
        bus.place(&hit, sample_at(t, SR), 0.3, Pan::Fixed(-0.2), 0.4);
    }
    let tone = partials(
        n,
        SR,
        220.0,
        &[Partial::new(1.0, 0.2), Partial::new(2.0, 0.1).decaying(0.5)],
    );
    bus.place(&tone, 0, 1.0, Pan::Fixed(0.0), 0.2);
    let rooms = (
        Room::new(1.0, 0.5, 4000.0, seed_for(seed, 1000))
            .unwrap()
            .impulse_response(SR),
        Room::new(1.0, 0.5, 4000.0, seed_for(seed, 1001))
            .unwrap()
            .impulse_response(SR),
    );
    let mut mix = bus.render((&rooms.0, &rooms.1), 0.8);
    let duck = duck_windows(
        n,
        SR,
        &[(0.4, 0.8)],
        &DuckShape {
            depth_db: -10.0,
            pre: 0.05,
            post: 0.1,
            ramp: 0.08,
        },
    )
    .unwrap();
    mix.apply_gain(&duck);
    fade(&mut mix, SR, 0.02, 0.2);
    normalize_peak(&mut mix, 0.89);
    let pcm = to_pcm16(&mix, 48_000);
    let dir = std::env::temp_dir().join(format!(
        "guion-synth-{}-{seed}-{extra_event}",
        std::process::id()
    ));
    let path = dir.join("a.wav");
    write_wav(&path, &pcm).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (bytes, hits.concat())
}

/// AC1: the same input gives the same bytes; one event's noise does not
/// depend on the others (adding an event leaves the rest's noise alone).
#[test]
fn ac1_deterministic_bytes_and_independent_streams() {
    let (a, ha) = scene(7, false);
    let (b, hb) = scene(7, false);
    assert_eq!(a, b, "two runs, two different files");
    let (c, hc) = scene(7, true);
    assert_ne!(a, c);
    assert_eq!(ha, hc, "adding an event changed another event's noise");
    assert_eq!(ha, hb);
    let (d, _) = scene(8, false);
    assert_ne!(a, d, "the seed must matter");
}

/// AC2: oscillators against their closed forms.
#[test]
fn ac2_oscillators() {
    let n = 200_000; // crosses many resync blocks
    let x = sine(n, SR, 1318.5, 0.7, 0.5);
    for i in (0..n).step_by(997) {
        let want = 0.5 * (TAU * 1318.5 * i as f64 / SR + 0.7).sin();
        assert!((x[i] - want).abs() < 1e-9, "sample {i}: {} vs {want}", x[i]);
    }
    let p = partials(
        48_000,
        SR,
        100.0,
        &[
            Partial::new(1.0, 1.0),
            Partial::new(2.756, 0.6).decaying(0.25).with_phase(1.0),
        ],
    );
    for i in [0usize, 1, 4_800, 12_000, 47_999] {
        let t = i as f64 / SR;
        let want =
            (TAU * 100.0 * t).sin() + 0.6 * (TAU * 275.6 * t + 1.0).sin() * (-t / 0.25).exp();
        assert!((p[i] - want).abs() < 1e-9);
    }
    // the phase of a frequency track is its running sum
    let f = [100.0, 200.0, 300.0];
    let ph = phase_of(&f, SR);
    assert!((ph[2] - TAU * 600.0 / SR).abs() < 1e-15);
}

/// AC3: envelopes at their defining points.
#[test]
fn ac3_envelopes() {
    let e = exp_decay(48_001, SR, 0.5);
    assert_eq!(e[0], 1.0);
    assert!((e[24_000] - (-1.0f64).exp()).abs() < 1e-12, "e^-1 at τ");
    assert_eq!(exp_decay(3, SR, 0.0), vec![1.0, 0.0, 0.0]);
    let adsr = Adsr {
        attack: 0.1,
        decay: 0.2,
        sustain: 0.5,
        release: 0.4,
    };
    let a = adsr.render((1.0 * SR) as usize, SR, 0.5);
    let at = |t: f64| a[(t * SR) as usize];
    assert!((at(0.05) - 0.5).abs() < 1e-3);
    assert!((at(0.1) - 1.0).abs() < 1e-3);
    assert!((at(0.2) - 0.75).abs() < 1e-3);
    assert!((at(0.4) - 0.5).abs() < 1e-9);
    assert!((at(0.7) - 0.25).abs() < 1e-3);
    assert_eq!(at(0.95), 0.0);
    let b = breakpoints(5, 1.0, &[(1.0, 2.0), (3.0, 4.0)]);
    assert_eq!(
        b,
        vec![2.0, 2.0, 3.0, 4.0, 4.0],
        "flat outside, linear inside"
    );
}

/// AC4: the noise is standard normal and repeatable.
#[test]
fn ac4_seeded_noise() {
    let x = Noise::new(42).white(200_000);
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
    assert!(mean.abs() < 0.01, "mean {mean}");
    assert!((var - 1.0).abs() < 0.02, "variance {var}");
    assert_eq!(x, Noise::new(42).white(200_000));
    assert_ne!(x[..10], Noise::new(43).white(10)[..]);
    assert_ne!(seed_for(1, 0), seed_for(1, 1));
    let mut u = Noise::new(5);
    assert!((0..10_000).all(|_| (0.0..1.0).contains(&u.uniform())));
}

/// Steady-state gain of a filter on a sine at `f`, away from the edges.
fn gain_on_sine(f: f64, run: impl Fn(&[f64]) -> Vec<f64>) -> f64 {
    let n = 96_000;
    let x = sine(n, SR, f, 0.0, 1.0);
    let y = run(&x);
    let rms = |v: &[f64]| (v.iter().map(|s| s * s).sum::<f64>() / v.len() as f64).sqrt();
    rms(&y[24_000..72_000]) / rms(&x[24_000..72_000])
}

/// AC5: every filter's response matches its closed form.
#[test]
fn ac5_filters_match_closed_forms() {
    for f in [60.0, 520.0, 2000.0] {
        let g = gain_on_sine(f, |x| filter::lowpass(x, SR, 520.0, 2));
        let want = filter::lowpass_gain(f, 520.0, 2);
        assert!(
            (g / want - 1.0).abs() < 0.005,
            "low-pass at {f}: {g} vs {want}"
        );
        let g = gain_on_sine(f, |x| filter::highpass(x, SR, 70.0, 2));
        let want = filter::highpass_gain(f, 70.0, 2);
        assert!(
            (g / want - 1.0).abs() < 0.005,
            "high-pass at {f}: {g} vs {want}"
        );
        let g = gain_on_sine(f, |x| filter::bandpass(x, SR, 900.0, 0.6));
        let want = filter::bandpass_gain(f, 900.0, 0.6);
        assert!((g / want - 1.0).abs() < 0.005, "band at {f}: {g} vs {want}");
    }
    // the band is 1 at its centre; the low-pass is −3 dB at its cutoff
    assert!((filter::bandpass_gain(900.0, 900.0, 4.0) - 1.0).abs() < 1e-15);
    assert!((filter::lowpass_gain(520.0, 520.0, 2) - 0.5f64.sqrt()).abs() < 1e-15);
    let p = OnePole::new(SR, 8.0);
    for f in [2.0, 8.0, 40.0] {
        let g = gain_on_sine(f, |x| p.lowpass(x));
        let want = p.gain(SR, f);
        assert!(
            (g / want - 1.0).abs() < 0.01,
            "one-pole at {f}: {g} vs {want}"
        );
    }
    // the spectral filter does not shift a click in time
    let mut click = vec![0.0; 4096];
    click[1000] = 1.0;
    let y = filter::lowpass(&click, SR, 2000.0, 2);
    let peak = y
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0;
    assert_eq!(peak, 1000);
    // and the transform behind it inverts
    let x: Vec<f64> = Noise::new(3).white(1000);
    let back = fft::irfft(&fft::rfft(&x, 1024), 1024);
    assert!(x.iter().zip(&back).all(|(a, b)| (a - b).abs() < 1e-12));
}

/// AC6: the room has unit energy, falls 60 dB per t60, and is level-stable:
/// a steady tone at any frequency passes with the target gain (within a few
/// dB at worst, about one on average), whatever the seed.
#[test]
fn ac6_room() {
    let room = Room::new(1.0, 2.0, 20_000.0, 9).unwrap();
    let h = room.impulse_response(SR);
    assert_eq!(h.len(), 96_000);
    let e: f64 = h.iter().map(|v| v * v).sum();
    assert!((e - 1.0).abs() < 1e-12, "energy {e}");
    // energy in 50 ms windows at 0.2 s and 0.7 s: 0.5 s apart, −30 dB
    let win = |t: f64| -> f64 {
        let a = (t * SR) as usize;
        h[a..a + 2400].iter().map(|v| v * v).sum::<f64>()
    };
    let drop = 10.0 * (win(0.7) / win(0.2)).log10();
    assert!((drop + 30.0).abs() < 1.5, "fell {drop} dB in half a t60");
    // an impulse heard in the room is the response
    let mut x = vec![0.0; 120_000];
    x[0] = 1.0;
    let y = room.apply(&x, SR);
    assert!(y.iter().zip(&h).all(|(a, b)| (a - b).abs() < 1e-12));
    assert_eq!(room.impulse_response(SR), h, "same seed, same room");

    // level-stable: |H(f)| against the target low-pass, at tones on and off any grid
    let mut devs = Vec::new();
    for seed in [1u64, 2, 3] {
        let h = Room::new(1.0, 1.2, 4200.0, seed)
            .unwrap()
            .impulse_response(SR);
        let freqs: Vec<f64> = (0..40)
            .map(|i| 30.0 * 100f64.powf(i as f64 / 39.0))
            .chain([55.0, 77.78, 98.0])
            .collect();
        let g: Vec<f64> = freqs
            .iter()
            .map(|f| {
                let (mut re, mut im) = (0.0, 0.0);
                for (i, v) in h.iter().enumerate() {
                    let a = TAU * f * i as f64 / SR;
                    re += v * a.cos();
                    im -= v * a.sin();
                }
                10.0 * ((re * re + im * im) / filter::lowpass_gain(*f, 4200.0, 2).powi(2)).log10()
            })
            .collect();
        devs.extend(g);
    }
    let mean =
        10.0 * (devs.iter().map(|d| 10f64.powf(d / 10.0)).sum::<f64>() / devs.len() as f64).log10();
    let rms = (devs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / devs.len() as f64).sqrt();
    let worst = devs.iter().fold(0.0_f64, |m, d| m.max((d - mean).abs()));
    assert!(
        rms < 1.5,
        "the room's gain wanders {rms:.2} dB RMS across tones and seeds"
    );
    assert!(worst < 6.0, "a tone is {worst:.1} dB off the room's level");
}

/// AC7: constant-power pan.
#[test]
fn ac7_constant_power_pan() {
    for p in [-1.0, -0.6, -0.2, 0.0, 0.3, 1.0] {
        let (l, r) = pan_gains(p);
        assert!((l * l + r * r - 1.0).abs() < 1e-15);
    }
    let (l, r) = pan_gains(0.0);
    assert!((l - r).abs() < 1e-15 && (l - (PI / 4.0).cos()).abs() < 1e-15);
    let (l, r) = pan_gains(-1.0);
    assert!((l - 1.0).abs() < 1e-15 && r.abs() < 1e-15);
    assert_eq!(pan_gains(3.0), pan_gains(1.0), "clamped");
}

/// AC8: a bus places at the sample, sends to the room and renders
/// dry + wet · room(send).
#[test]
fn ac8_bus() {
    let mut bus = Bus::new(10);
    bus.place(&[1.0, 2.0], 3, 0.5, Pan::Fixed(0.0), 0.25);
    bus.place(&[1.0, 1.0, 1.0], -1, 1.0, Pan::Track(&[-1.0, 1.0]), 0.0);
    bus.place(&[9.0], 10, 1.0, Pan::Fixed(0.0), 1.0); // past the end: nothing
    let c = (PI / 4.0).cos();
    assert!((bus.dry.l[3] - 0.5 * c).abs() < 1e-15 && (bus.dry.r[4] - c).abs() < 1e-15);
    assert!((bus.send.l[4] - 0.25 * c).abs() < 1e-15);
    // the track pan follows the signal, not the bus: sample 1 of the signal is at 0
    assert!(bus.dry.l[0].abs() < 1e-15 && (bus.dry.r[0] - 1.0).abs() < 1e-15);
    assert!(bus.dry.l[2].abs() < 1e-15, "last pan holds");
    assert_eq!(bus.dry.l[9], 0.0);
    assert_eq!(sample_at(0.5 / SR, SR), 0, "ties to even");
    assert_eq!(sample_at(1.5 / SR, SR), 2);
    let room = Room::new(0.5, 0.1, 8000.0, 1).unwrap();
    let h = room.impulse_response(SR);
    let out = bus.render((&h, &h), 0.8);
    let wet = room.apply(&bus.send.l, SR);
    for ((o, d), w) in out.l.iter().zip(&bus.dry.l).zip(&wet) {
        assert!((o - (d + 0.8 * w)).abs() < 1e-15);
    }
}

/// AC9: ducking by the line windows and by an envelope follower.
#[test]
fn ac9_ducking() {
    let n = (4.0 * SR) as usize;
    let shape = DuckShape {
        depth_db: -18.0,
        pre: 0.08,
        post: 0.12,
        ramp: 0.08,
    };
    let g = duck_windows(n, SR, &[(1.0, 2.0)], &shape).unwrap();
    let at = |t: f64| g[(t * SR) as usize];
    let low = 10f64.powf(-18.0 / 20.0);
    assert!((at(0.5) - 1.0).abs() < 1e-9 && (at(3.5) - 1.0).abs() < 1e-9);
    assert!((at(1.5) - low).abs() < 1e-9, "{} vs {low}", at(1.5));
    assert!(
        (at(0.92) - (1.0 + low) / 2.0).abs() < 0.01,
        "half way at the window's edge"
    );
    assert!((at(0.0) - 1.0).abs() < 1e-9, "no sag at the start");
    // the follower finds a tone burst from 1 s to 2 s
    let mut voice = vec![0.0; n];
    synth::add_sine(
        &mut voice[(SR as usize)..(2 * SR as usize)],
        SR,
        200.0,
        0.0,
        0.5,
    );
    let f = Follower {
        attack: 50.0,
        release: 5.0,
        threshold_db: -30.0,
    };
    let gf = duck_follower(&voice, SR, &f, &shape).unwrap();
    let atf = |t: f64| gf[(t * SR) as usize];
    assert!(
        (atf(1.5) - low).abs() < 1e-9
            && (atf(0.5) - 1.0).abs() < 1e-9
            && (atf(3.5) - 1.0).abs() < 1e-9
    );
}

/// AC10: 48 kHz, stereo, 16-bit, canonical header, exact round trip.
#[test]
fn ac10_stereo_wav() {
    let mut x = Stereo::silence(4);
    x.l = vec![0.0, 0.5, -1.0, 2.0];
    x.r = vec![1.0 / 32767.0 * 0.5, -0.25, 1.0, -2.0];
    let pcm = to_pcm16(&x, 48_000);
    assert_eq!(
        pcm.samples,
        vec![0, 0, 16384, -8192, -32767, 32767, 32767, -32768]
    );
    assert_eq!(to_i16(1.5 / 32767.0), 2, "ties to even");
    let dir = std::env::temp_dir().join(format!("guion-synth-wav-{}", std::process::id()));
    let path = dir.join("s.wav");
    write_wav(&path, &pcm).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 44 + 16);
    assert_eq!(&bytes[22..24], &2u16.to_le_bytes(), "two channels");
    assert_eq!(&bytes[28..32], &(48_000u32 * 4).to_le_bytes(), "byte rate");
    assert_eq!(&bytes[32..34], &4u16.to_le_bytes(), "block align");
    assert_eq!(read_wav_pcm16(&bytes).unwrap(), pcm);
    let _ = std::fs::remove_dir_all(&dir);
}

/// AC12: bad parameters are typed and named, and nothing is written.
#[test]
fn ac12_failures_name_the_parameter() {
    match Room::new(0.0, 1.0, 1000.0, 1) {
        Err(AudioError::InvalidSynth { what, .. }) => assert_eq!(what, "room.t60"),
        other => panic!("{other:?}"),
    }
    let bad = DuckShape {
        depth_db: f64::NAN,
        pre: 0.0,
        post: 0.0,
        ramp: 0.0,
    };
    let e = duck_windows(10, SR, &[], &bad).unwrap_err();
    assert!(e.to_string().contains("duck.depth_db"), "{e}");
    let bad = DuckShape {
        depth_db: -6.0,
        pre: 0.0,
        post: 0.0,
        ramp: -1.0,
    };
    assert!(duck_windows(10, SR, &[], &bad)
        .unwrap_err()
        .to_string()
        .contains("duck.ramp"));
    let f = Follower {
        attack: 0.0,
        release: 1.0,
        threshold_db: -30.0,
    };
    let ok = DuckShape {
        depth_db: -6.0,
        pre: 0.0,
        post: 0.0,
        ramp: 0.0,
    };
    assert!(duck_follower(&[0.0; 4], SR, &f, &ok)
        .unwrap_err()
        .to_string()
        .contains("follower.attack"));
    let odd = guion_audio::Pcm16 {
        sample_rate: 48_000,
        channels: 2,
        samples: vec![1, 2, 3],
    };
    let path = std::env::temp_dir().join(format!("guion-synth-odd-{}.wav", std::process::id()));
    assert!(write_wav(&path, &odd).is_err());
    assert!(!path.exists(), "nothing written on failure");
}

/// The fades and the peak.
#[test]
fn fades_and_peak() {
    let mut x = Stereo::dual(&vec![1.0; 48_000]);
    fade(&mut x, SR, 0.5, 0.25);
    assert!((x.l[12_000] - 0.5).abs() < 1e-12);
    assert!((x.l[42_000] - 0.5).abs() < 1e-12);
    let k = normalize_peak(&mut x, 0.89);
    assert!((x.peak() - 0.89).abs() < 1e-12 && k > 0.0);
}
