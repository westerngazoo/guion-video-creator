use guion_assemble::duration;

use crate::error::AudioError;
use crate::wav::{WavBuffer, SAMPLE_RATE};

/// Generate a simple 8-bit-style bed for known generator names.
pub fn generate_bed(generator: &str, total_secs: f64, fps: f64) -> Result<WavBuffer, AudioError> {
    match generator {
        "fbf-default" | "audio8" => Ok(synth_fbf_default(total_secs, fps)),
        other => Err(AudioError::Engine(format!(
            "generador de audio desconocido: {other} (soportado: fbf-default)"
        ))),
    }
}

fn synth_fbf_default(total_secs: f64, fps: f64) -> WavBuffer {
    let sr = SAMPLE_RATE;
    let n = (total_secs * sr as f64).ceil() as usize;
    let mut buf = vec![0.0f32; n];
    let octava = total_secs / 32.0;
    let bajos = [123.47, 123.47, 123.47, 123.47];
    let arps = [246.94, 293.66, 369.99];
    let patron = [0usize, 1, 2, 1, 0, 1, 2, 1];

    for (barra, &bajo) in bajos.iter().enumerate() {
        for e in 0..8 {
            let t = (barra * 8 + e) as f64 * octava;
            let f = bajo * if e % 2 == 0 { 1.0 } else { 2.0 };
            mix_samples(&mut buf, &square(f, octava * 0.92, 0.16, 0.5, 3.0), t, sr);
            let fa = arps[patron[e]];
            mix_samples(&mut buf, &square(fa, octava * 0.82, 0.13, 0.25, 4.0), t, sr);
            if e % 2 == 1 {
                mix_samples(&mut buf, &noise(0.030, 0.07), t, sr);
            }
        }
        for beat in [0, 4] {
            mix_samples(
                &mut buf,
                &square(65.4, 0.06, 0.20, 0.5, 8.0),
                (barra * 8 + beat) as f64 * octava,
                sr,
            );
        }
    }

    let fade = 220.min(buf.len() / 2);
    let len = buf.len();
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        buf[i] *= g;
        buf[len - 1 - i] *= g;
    }

    let _ = fps; // reserved for frame-synced variants
    WavBuffer {
        samples: buf,
        sample_rate: sr,
    }
}

pub fn bed_for_screenplay(
    sp: &guion_core::Screenplay,
    generator: &str,
) -> Result<WavBuffer, AudioError> {
    let total = timeline_duration(sp);
    generate_bed(generator, total, sp.meta.fps)
}

pub fn timeline_duration(sp: &guion_core::Screenplay) -> f64 {
    let base = duration(sp);
    let narr_end = sp
        .narration
        .iter()
        .map(|n| n.at.end)
        .fold(0.0_f64, f64::max);
    base.max(narr_end).max(0.5)
}

fn mix_samples(buf: &mut [f32], samples: &[f32], t0: f64, sr: u32) {
    let i0 = (t0 * sr as f64) as usize;
    for (j, s) in samples.iter().enumerate() {
        let k = i0 + j;
        if k < buf.len() {
            buf[k] = (buf[k] + *s).clamp(-1.0, 1.0);
        }
    }
}

fn square(freq: f64, dur: f64, vol: f32, duty: f64, decay: f64) -> Vec<f32> {
    let n = (SAMPLE_RATE as f64 * dur).ceil() as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / SAMPLE_RATE as f64;
        let ph = (t * freq) % 1.0;
        let s = vol * if ph < duty { 1.0 } else { -1.0 };
        let env = (-decay * t / dur).exp() as f32;
        out.push(s * env);
    }
    out
}

fn noise(dur: f64, vol: f32) -> Vec<f32> {
    let n = (SAMPLE_RATE as f64 * dur).ceil() as usize;
    let mut reg: u16 = 0x4C1B;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let bit = (reg ^ (reg >> 1)) & 1;
        reg = (reg >> 1) | (bit << 14);
        let env = (-9.0 * i as f64 / n as f64).exp() as f32;
        let s = vol * if reg & 1 != 0 { 1.0 } else { -1.0 };
        out.push(s * env);
    }
    out
}
