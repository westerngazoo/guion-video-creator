use std::path::Path;
use std::process::Command;

use crate::error::AudioError;
use crate::wav::SAMPLE_RATE;

/// Synthesizes speech samples for one narration cue.
pub trait NarrationEngine {
    fn name(&self) -> &'static str;

    /// Return mono samples at [`SAMPLE_RATE`]. Duration may differ from the cue window.
    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError>;
}

/// MVP engine: short tone bursts proportional to cue length (no external deps).
pub struct ScaffoldEngine;

impl NarrationEngine for ScaffoldEngine {
    fn name(&self) -> &'static str {
        "scaffold"
    }

    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError> {
        let words = text.split_whitespace().count().max(1);
        let dur = (words as f64 * 0.18).clamp(0.35, 6.0);
        Ok(tone_burst(dur, 440.0, 0.12))
    }
}

/// Offline TTS via the `piper` CLI when installed.
pub struct PiperEngine {
    model: String,
}

impl PiperEngine {
    pub fn new(model: impl Into<String>) -> Self {
        PiperEngine {
            model: model.into(),
        }
    }
}

impl NarrationEngine for PiperEngine {
    fn name(&self) -> &'static str {
        "piper"
    }

    fn synthesize(&self, text: &str) -> Result<Vec<f32>, AudioError> {
        let piper = which_piper()?;
        let tmp = std::env::temp_dir().join(format!("guion-piper-{}.wav", std::process::id()));
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "printf '%s' '{}' | {} --model '{}' --output_file '{}'",
                text.replace('\'', "'\\''"),
                piper,
                self.model,
                tmp.display()
            ))
            .output()
            .map_err(|e| AudioError::Engine(e.to_string()))?;
        if !out.status.success() {
            return Err(AudioError::Engine(
                String::from_utf8_lossy(&out.stderr).to_string(),
            ));
        }
        let samples = read_wav_mono(&tmp)?;
        let _ = std::fs::remove_file(&tmp);
        Ok(samples)
    }
}

pub fn engine_by_name(
    name: &str,
    piper_model: Option<&str>,
) -> Result<Box<dyn NarrationEngine>, AudioError> {
    match name {
        "scaffold" => Ok(Box::new(ScaffoldEngine)),
        "piper" => Ok(Box::new(PiperEngine::new(
            piper_model.unwrap_or("es_ES-sharvina-medium.onnx"),
        ))),
        other => Err(AudioError::Engine(format!(
            "motor desconocido: {other} (usa scaffold o piper)"
        ))),
    }
}

fn which_piper() -> Result<String, AudioError> {
    let out = Command::new("which")
        .arg("piper")
        .output()
        .map_err(|e| AudioError::Engine(e.to_string()))?;
    if !out.status.success() {
        return Err(AudioError::PiperMissing);
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn tone_burst(dur: f64, freq: f64, vol: f32) -> Vec<f32> {
    let n = (dur * SAMPLE_RATE as f64).ceil() as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / SAMPLE_RATE as f64;
        let env = ((i as f64 / n as f64) * std::f64::consts::PI).sin();
        let s = vol * (2.0 * std::f64::consts::PI * freq * t).sin() as f32 * env as f32;
        out.push(s);
    }
    out
}

fn read_wav_mono(path: &Path) -> Result<Vec<f32>, AudioError> {
    let data = std::fs::read(path)?;
    if data.len() < 44 || &data[0..4] != b"RIFF" {
        return Err(AudioError::Engine("wav inválido de piper".into()));
    }
    let channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let bits = u16::from_le_bytes([data[34], data[35]]);
    let data_off = 36 + 8; // naive: assume standard 44-byte header
    let pcm = &data[data_off..];
    let mut mono = Vec::new();
    if bits == 16 {
        for chunk in pcm.chunks(2 * channels) {
            let s = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0;
            mono.push(s);
        }
    } else {
        return Err(AudioError::Engine("solo wav PCM 16-bit".into()));
    }
    Ok(mono)
}
