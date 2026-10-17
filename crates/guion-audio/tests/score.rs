//! R-0011: the sound design as data. One test per acceptance criterion, with
//! test instruments only (no brand lives in this crate).

use std::sync::{Arc, Mutex};

use guion_audio::score::{
    hits_tsv, merge_lines, render, render_post, sound, sound_lasting, Instruments, Kind, Rendered,
    Score, Source, Timeline, Voice, RATE,
};
use guion_audio::synth::{exp_decay, sine, Noise};
use guion_audio::{read_wav_pcm16, write_wav_pcm16, AudioError};

fn times() -> Timeline {
    Timeline::from_json(
        r#"{"duracion": 3.0, "fps": 30.0, "eventos": {"a": 0.5, "q2-fin": 1.0, "menos": 2.0, "menos-1": 2.5},
            "series": {"v": {"t0": 0.0, "dt": 0.5, "v": [0.0, 0.25, 0.5, 0.75]}}}"#,
    )
    .unwrap()
}

/// A tick (a seeded noise burst), a tone (f0 with a decay), a swell (lasts
/// `dur`), a hum (drone) and a glide (follows the series).
fn kit() -> Instruments {
    let mut r = Instruments::new();
    r.add(Kind::Hit, "tick", |_| {
        Ok(sound(|cx| {
            let m = 4_800;
            let e = exp_decay(m, RATE, 0.01);
            cx.noise
                .white(m)
                .iter()
                .zip(e)
                .map(|(a, b)| a * b)
                .collect::<Vec<_>>()
                .into()
        }))
    });
    r.add(Kind::Hit, "tone", |d| {
        let (f0, dur) = (d.frequency(440.0)?, d.duration(0.5)?);
        Ok(sound(move |_| {
            sine((dur * RATE) as usize, RATE, f0, 0.0, 1.0).into()
        }))
    });
    r.add(Kind::Hit, "swell", |d| {
        let dur = d.duration(0.8)?;
        Ok(sound_lasting(dur, move |cx| {
            let m = (dur * RATE) as usize;
            let k: Vec<f64> = (0..m).map(|i| i as f64 / m as f64).collect();
            let s = cx
                .noise
                .white(m)
                .iter()
                .zip(&k)
                .map(|(a, b)| a * b)
                .collect();
            Rendered {
                samples: s,
                pan_curve: Some(k),
            }
        }))
    });
    r.add(Kind::Drone, "hum", |d| {
        let f0 = d.positive("f0", 55.0)?;
        Ok(sound(move |cx| sine(cx.len, RATE, f0, 0.0, 1.0).into()))
    });
    r.add(Kind::Follower, "glide", |_| {
        Ok(sound(|cx| {
            cx.series
                .iter()
                .map(|v| (v * 1000.0).sin())
                .collect::<Vec<_>>()
                .into()
        }))
    });
    r.set_default(Kind::Drone, "hum");
    r
}

fn parse(s: &str) -> Result<Score, String> {
    Score::parse(s, &times(), &kit())
}

fn hits(d: &Score) -> Vec<(f64, f64, f64)> {
    d.pieces
        .iter()
        .filter_map(|p| match &p.source {
            Source::Hit { at, .. } => Some((*at, p.pan, p.gain)),
            _ => None,
        })
        .collect()
}

#[test]
fn ac1_no_brand_and_the_whole_format_parses() {
    assert!(Instruments::new().names(Kind::Hit).is_empty());
    let d = parse(
        "[sonido]\nsemilla = 2\nsala = { t60 = 1.5, largo = 2.0, oscuro = 3000.0, envio = 0.5 }\n\
         [mezcla]\ncama_rms = -12.0\npico = 0.8\nentrada = 0.0\nsalida = 0.2\n\
         [[dron]]\nf0 = 110\nenvolvente = [[\"0\", 0.0], [\"a\", 1.0], [\"fin\", 0.0]]\nlfo = { hz = 0.5 }\n\
         [[seguidor]]\nserie = \"v\"\ninstrumento = \"glide\"\ndesde = \"a\"\nhasta = \"menos\"\n\
         [[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\nbus = \"seco\"\n",
    )
    .unwrap();
    assert_eq!(d.pieces.len(), 3);
    assert_eq!(d.pieces[0].instrument, "hum", "the caller's default");
    assert_eq!(
        (
            d.pieces[0].bus.as_str(),
            d.pieces[1].bus.as_str(),
            d.pieces[2].bus.as_str()
        ),
        ("dron", "dron", "seco")
    );
    assert!((d.room.t60, d.mix.peak) == (1.5, 0.8));
    let e = Score::parse(
        "[sonido]\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\n",
        &times(),
        &Instruments::new(),
    )
    .unwrap_err();
    assert!(e.contains("instrumento «tick» (valen: )"), "{e}");
}

#[test]
fn ac2_times_are_named_events() {
    let t = times();
    assert_eq!(t.time("a").unwrap(), 0.5);
    assert_eq!(t.time("a - 0.25").unwrap(), 0.25);
    assert_eq!(t.time("q2-fin + 0.5").unwrap(), 1.5);
    assert_eq!(t.time("q2-fin+0.5").unwrap(), 1.5);
    assert_eq!(t.time("menos-1").unwrap(), 2.5, "the exact name wins");
    assert_eq!(t.time("menos - 1").unwrap(), 1.0);
    assert_eq!(t.time("fin - 0.5").unwrap(), 2.5);
    assert_eq!(t.time("0").unwrap(), 0.0);
    assert!((t.time("a..menos @ 0.25").unwrap() - 0.875).abs() < 1e-12);
    assert!((t.time("a .. q2-fin + 0.5 @ 0.5").unwrap() - 1.0).abs() < 1e-12);
    assert!(t.time("a @ 0.5").is_err());
    assert!(t.time("qde").unwrap_err().contains("qde"));
    let m = t.series("v").unwrap().samples(144_000);
    assert!((m[12_000] - 0.125).abs() < 1e-12 && m[143_999] == 0.75);
    // the first of a repeated event wins, and numbers parse exactly
    let t =
        Timeline::from_json(r#"{"duracion": 1, "eventos": {"x": 0.1, "x": 0.9, "y": 86.166667}}"#)
            .unwrap();
    assert_eq!(
        (t.event("x"), t.event("y"), t.fps),
        (Some(0.1), Some("86.166667".parse().unwrap()), 30.0)
    );
}

#[test]
fn ac3_unknown_keys_and_errors_name_their_table() {
    const MIN: &str =
        "[sonido]\nsemilla = 3\n[[golpe]]\nen = \"a\"\ninstrumento = \"tone\"\nf0 = 440\n";
    assert!(parse(MIN).is_ok());
    let e = parse(&format!("{MIN}f00 = 1\n")).unwrap_err();
    assert!(
        e.contains("[[golpe]] 1") && e.contains("f00") && e.contains("octavas"),
        "{e}"
    );
    let e = parse("[sonido]\nsemila = 1\n").unwrap_err();
    assert!(e.contains("semila"), "{e}");
    let e = parse("[sonido]\n[[golpe]]\nen = \"qde\"\ninstrumento = \"tick\"\n").unwrap_err();
    assert!(e.contains("[[golpe]] 1") && e.contains("qde"), "{e}");
    let e = parse("[sonido]\n[[golpe]]\nen = \"a\"\nhasta = \"a\"\ninstrumento = \"swell\"\n")
        .unwrap_err();
    assert!(e.contains("hasta"), "{e}");
    let e = parse(
        "[sonido]\n[[golpe]]\nen = \"a\"\nhasta = \"menos\"\ndur = 1\ninstrumento = \"swell\"\n",
    )
    .unwrap_err();
    assert!(e.contains("dur o hasta"), "{e}");
    let e = parse("[sonido]\n[[seguidor]]\nserie = \"w\"\ninstrumento = \"glide\"\n").unwrap_err();
    assert!(e.contains("[[seguidor]] 1") && e.contains("«w»"), "{e}");
    let e = parse("[sonido]\n[[seguidor]]\nserie = \"v\"\ninstrumento = \"glide\"\nenvolvente = [[\"menos\", 1.0], [\"a\", 0.0]]\n").unwrap_err();
    assert!(e.contains("hacia atrás"), "{e}");
    let e =
        parse("[sonido]\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\npan = 1.5\n").unwrap_err();
    assert!(e.contains("pan = 1.5"), "{e}");
    let e =
        parse("[sonido]\n[agachar]\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\n").unwrap_err();
    assert!(e.contains("sin [voz]"), "{e}");
}

#[test]
fn ac4_repeated_hits_lists_steps_and_repetir() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut r = kit();
    let s2 = seen.clone();
    r.add(Kind::Hit, "bell", move |d| {
        s2.lock().unwrap().push(d.frequency(220.0)?);
        Ok(sound(|_| vec![1.0].into()))
    });
    let t = times();
    let d = Score::parse(
        "[sonido]\n[[golpe]]\nen = [\"a\", \"menos\", \"menos-1\"]\ninstrumento = \"bell\"\nf0 = 220\noctavas = 0.0\npaso = { octavas = 1.0 }\npan = [0.5, 0.0, -0.5]\n",
        &t,
        &r,
    )
    .unwrap();
    assert_eq!(*seen.lock().unwrap(), vec![220.0, 440.0, 880.0]);
    assert_eq!(
        hits(&d).iter().map(|h| h.1).collect::<Vec<_>>(),
        vec![0.5, 0.0, -0.5]
    );
    assert_eq!(d.pieces[1].location, "[[golpe]] 1 («menos»)");
    let e = parse(
        "[sonido]\n[[golpe]]\nen = [\"a\", \"menos\"]\ninstrumento = \"tick\"\npan = [0.5]\n",
    )
    .unwrap_err();
    assert!(e.contains("pan tiene 1 valores para 2 golpes"), "{e}");
    let e = parse("[sonido]\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\npan = [0.5]\n")
        .unwrap_err();
    assert!(e.contains("un solo valor"), "{e}");
    let e = parse("[sonido]\n[[golpe]]\nen = [\"a\", \"menos\"]\ninstrumento = \"tick\"\npaso = { octavas = 1.0 }\n").unwrap_err();
    assert!(e.contains("paso.octavas"), "{e}");
    // n spread from desde to before hasta; every cada until hasta, scaled by the envelope
    let d = parse("[sonido]\n[[golpe]]\nrepetir = { desde = \"a\", hasta = \"menos\", n = 3 }\ninstrumento = \"tick\"\n").unwrap();
    assert_eq!(
        hits(&d).iter().map(|h| h.0).collect::<Vec<_>>(),
        vec![0.5, 1.0, 1.5]
    );
    let d = parse("[sonido]\n[[golpe]]\nrepetir = { desde = \"a\", hasta = \"menos\", cada = 0.4 }\ninstrumento = \"tick\"\nganancia = 2.0\nenvolvente = [[\"a\", 0.25], [\"menos\", 0.75]]\n").unwrap();
    let h = hits(&d);
    assert_eq!(h.len(), 4);
    for (k, (at, _, gain)) in h.iter().enumerate() {
        assert!((at - (0.5 + 0.4 * k as f64)).abs() < 1e-12);
        assert!((gain - 2.0 * (0.25 + 0.5 * (at - 0.5) / 1.5)).abs() < 1e-12);
    }
    let d = parse("[sonido]\n[[golpe]]\nrepetir = { desde = \"0\", cada_cuadros = 3, n = 2 }\ninstrumento = \"tick\"\n").unwrap();
    assert_eq!(
        hits(&d).iter().map(|h| h.0).collect::<Vec<_>>(),
        vec![0.0, 0.1]
    );
    let e = parse("[sonido]\n[[golpe]]\nen = \"a\"\nrepetir = { desde = \"a\", n = 2, cada = 1 }\ninstrumento = \"tick\"\n").unwrap_err();
    assert!(e.contains("no los dos"), "{e}");
    let e = parse("[sonido]\n[[golpe]]\nrepetir = { desde = \"0\", hasta = \"fin\", cada = 0.01 }\ninstrumento = \"tick\"\n").unwrap_err();
    assert!(e.contains("el máximo es 100"), "{e}");
}

#[test]
fn ac5_instruments_by_name_seeds_by_place_extent_in_tsv() {
    let e = parse("[sonido]\n[[golpe]]\nen = \"a\"\ninstrumento = \"gong\"\n").unwrap_err();
    assert!(
        e.contains("instrumento «gong» (valen: tick, tone, swell)"),
        "{e}"
    );
    let e = parse("[sonido]\n[[seguidor]]\nserie = \"v\"\n").unwrap_err();
    assert!(e.contains("falta instrumento"), "{e}");
    let d =
        parse("[sonido]\n[[golpe]]\nen = [\"a\", \"menos\"]\ninstrumento = \"tick\"\n").unwrap();
    assert_ne!(d.pieces[0].seed, d.pieces[1].seed);
    let d2 = parse("[sonido]\n[[golpe]]\nen = [\"a\", \"menos\"]\ninstrumento = \"tick\"\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\n").unwrap();
    assert_eq!(
        d.pieces[1].seed, d2.pieces[1].seed,
        "adding a table keeps the others' noise"
    );
    let d = parse("[sonido]\n[[golpe]]\nen = \"a\"\nhasta = \"menos\"\ninstrumento = \"swell\"\n[[golpe]]\nen = \"q2-fin\"\ninstrumento = \"tick\"\n").unwrap();
    assert_eq!(
        hits_tsv(&d),
        "instrumento\ten\tfin\nswell\t0.500000\t2.000000\ntick\t1.000000\t\n"
    );
}

#[test]
fn ac6_deterministic_at_its_sample_with_its_pan_curve() {
    let t = times();
    let src = "[sonido]\nsemilla = 3\n[[golpe]]\nen = \"a\"\ninstrumento = \"tone\"\nsala = 0.0\n[[golpe]]\nen = \"q2-fin\"\nhasta = \"menos\"\ninstrumento = \"swell\"\npan = -1.0\npan_hasta = 1.0\nsala = 0.0\n";
    let d = parse(src).unwrap();
    let (a, b) = (render(&d, &t, None).unwrap(), render(&d, &t, None).unwrap());
    assert_eq!(a.mix, b.mix);
    assert!(a.mix.peak() <= 0.89 + 1e-12);
    let dry = &a.dry["golpe"];
    // a sine with phase 0 starts at 0: the first non-zero sample is the next one
    assert_eq!(dry.l.iter().position(|v| v.abs() > 0.0).unwrap(), 24_001);
    // the swell pans hard left at its start (curve 0) and to the right at its end
    let (i0, i1) = (48_010, 95_990);
    assert!(dry.r[i0].abs() < 1e-3 * dry.l[i0].abs().max(1e-9) + 1e-12);
    assert!(dry.r[i1].abs() > 10.0 * dry.l[i1].abs());
    let other = Score::parse(&src.replace("semilla = 3", "semilla = 4"), &t, &kit()).unwrap();
    assert_ne!(render(&other, &t, None).unwrap().mix, a.mix);
}

fn voice(n: usize, lines: &[(f64, f64)]) -> Vec<f64> {
    let mut v = vec![0.0; n];
    let mut g = Noise::new(9);
    for &(a, b) in lines {
        for x in v
            .iter_mut()
            .take((b * RATE) as usize)
            .skip((a * RATE) as usize)
        {
            *x = 0.5 * g.gaussian().clamp(-1.9, 1.9);
        }
    }
    v
}

#[test]
fn ac7_the_bed_ducks_under_lines_and_merge_closes_short_gaps() {
    let l = [(0.5, 2.0), (2.35, 4.0), (4.55, 6.0)];
    assert_eq!(merge_lines(&l, 0.5), vec![(0.5, 4.0), (4.55, 6.0)]);
    assert_eq!(merge_lines(&l, 0.0), l.to_vec());
    let t = times();
    let lines = [(0.5, 1.2), (1.4, 2.0)];
    let v = voice(144_000, &lines);
    let base = "[sonido]\n[voz]\n[[dron]]\nsala = 0.0\n[agachar]\nantes = 0.0\ndespues = 0.0\nrampa = 0.01\nbuses = { dron = -20.0 }\n";
    let level = |src: &str, at: f64| {
        let d = Score::parse(src, &t, &kit()).unwrap();
        let o = render(
            &d,
            &t,
            Some(Voice {
                samples: &v,
                lines: &lines,
            }),
        )
        .unwrap();
        let i = (at * RATE) as usize;
        let rms = |x: &[f64]| (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
        (rms(&o.bed.l[i - 2_400..i + 2_400]), o.report)
    };
    let (inside, report) = level(base, 0.85);
    let (gap, _) = level(base, 1.3);
    let (outside, _) = level(base, 2.6);
    assert!(
        (20.0 * (inside / outside).log10() + 20.0).abs() < 0.5,
        "{}",
        20.0 * (inside / outside).log10()
    );
    assert!(
        gap > 5.0 * inside,
        "the bed opens between lines without unir"
    );
    let (merged, _) = level(&format!("{base}unir = 0.5\n"), 1.3);
    assert!((merged / inside - 1.0).abs() < 0.1, "unir keeps it down");
    assert!(report.contains("cama bajo la voz"), "{report}");
}

#[test]
fn ac8_the_bed_level_by_peak_or_rms() {
    let t = times();
    let lines = [(0.5, 2.5)];
    let v = voice(144_000, &lines);
    let run = |mix: &str| {
        let d = Score::parse(&format!("[sonido]\n[voz]\nnivel = 1.0\nsala = {{ nivel = 0.0 }}\n[mezcla]\n{mix}\nentrada = 0.0\nsalida = 0.0\n[[dron]]\n"), &t, &kit()).unwrap();
        render(
            &d,
            &t,
            Some(Voice {
                samples: &v,
                lines: &lines,
            }),
        )
        .unwrap()
    };
    let rms = |x: &guion_audio::synth::Stereo| {
        (x.l.iter().chain(&x.r).map(|v| v * v).sum::<f64>() / (2 * x.len()) as f64).sqrt()
    };
    let o = run("cama_rms = -12.0");
    assert!((20.0 * (rms(&o.bed) / rms(&o.voice)).log10() + 12.0).abs() < 1e-6);
    let o = run("cama = 0.3");
    // the voice enters at peak 1 (level 1, no room): the bed's peak is 0.3 of it
    assert!(
        (o.bed.peak() / o.voice.peak() - 0.3).abs() < 1e-9,
        "{}",
        o.bed.peak() / o.voice.peak()
    );
    assert!(Score::parse(
        "[sonido]\n[mezcla]\ncama = 0.3\ncama_rms = -3.0\n",
        &t,
        &kit()
    )
    .unwrap_err()
    .contains("no los dos"));
    assert!(Score::parse("[sonido]\n[mezcla]\npico = 1.2\n", &t, &kit())
        .unwrap_err()
        .contains("recortaría"));
}

#[test]
fn ac9_a_post_folder_to_audio_wav_or_nothing() {
    let dir = std::env::temp_dir().join(format!("guion-score-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("eventos.json"),
        r#"{"duracion": 2.0, "eventos": {"a": 0.5}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.join("voz.json"),
        r#"{"duracion_voz": 2.0, "lineas": [{"id": "x", "t0": 0.5, "t1": 1.5}]}"#,
    )
    .unwrap();
    let v: Vec<i16> = voice(96_000, &[(0.5, 1.5)])
        .iter()
        .map(|x| (x * 20_000.0) as i16)
        .collect();
    write_wav_pcm16(&dir.join("voz.wav"), 48_000, &v).unwrap();
    // a typo: nothing is written
    std::fs::write(
        dir.join("sonido.toml"),
        "[sonido]\n[voz]\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\nganacia = 1\n",
    )
    .unwrap();
    let e = render_post(&dir, &kit(), None, None).unwrap_err();
    assert!(
        matches!(&e, AudioError::InvalidScore(m) if m.contains("sonido.toml") && m.contains("ganacia")),
        "{e}"
    );
    assert!(!dir.join("audio.wav").exists());
    std::fs::write(dir.join("sonido.toml"), "[sonido]\n[voz]\n[agachar]\nbuses = { golpe = -6.0 }\n[[golpe]]\nen = \"a\"\ninstrumento = \"tick\"\n").unwrap();
    let stems = dir.join("pistas");
    let out = render_post(&dir, &kit(), None, Some(&stems)).unwrap();
    assert_eq!(
        (out.path.clone(), out.duration),
        (dir.join("audio.wav"), 2.0)
    );
    let a = std::fs::read(&out.path).unwrap();
    let pcm = read_wav_pcm16(&a).unwrap();
    assert_eq!(
        (pcm.sample_rate, pcm.channels, pcm.samples.len()),
        (48_000, 2, 2 * 96_000)
    );
    for f in [
        "voz.f32",
        "cama.f32",
        "golpe.f32",
        "golpe-seco.f32",
        "golpes.tsv",
    ] {
        assert!(stems.join(f).exists(), "{f}");
    }
    render_post(&dir, &kit(), None, None).unwrap();
    assert_eq!(std::fs::read(&out.path).unwrap(), a, "the same bytes twice");
    std::fs::remove_dir_all(&dir).unwrap();
}
