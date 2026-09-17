use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::error::AudioError;

pub const SAMPLE_RATE: u32 = 44_100;

/// Mono floating-point samples in [-1, 1].
#[derive(Debug, Clone)]
pub struct WavBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl WavBuffer {
    pub fn silence(seconds: f64, sample_rate: u32) -> Self {
        let n = (seconds * sample_rate as f64).ceil().max(0.0) as usize;
        WavBuffer {
            samples: vec![0.0; n],
            sample_rate,
        }
    }

    pub fn duration_secs(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }

    pub fn mix_at(&mut self, offset_secs: f64, segment: &[f32]) {
        let i0 = (offset_secs * self.sample_rate as f64).round() as isize;
        for (j, &s) in segment.iter().enumerate() {
            let k = i0 + j as isize;
            if k >= 0 {
                let k = k as usize;
                if k < self.samples.len() {
                    self.samples[k] = (self.samples[k] + s).clamp(-1.0, 1.0);
                }
            }
        }
    }

    pub fn apply_fade(&mut self, fade_samples: usize) {
        let n = fade_samples.min(self.samples.len() / 2);
        for i in 0..n {
            let g = i as f32 / n as f32;
            self.samples[i] *= g;
            let tail = self.samples.len() - 1 - i;
            self.samples[tail] *= g;
        }
    }

    pub fn write_wav(&self, path: &Path) -> Result<(), AudioError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut w = BufWriter::new(file);
        let data_len = self.samples.len() * 2;
        let byte_rate = self.sample_rate * 2;
        w.write_all(b"RIFF")?;
        w.write_all(&(36u32 + data_len as u32).to_le_bytes())?;
        w.write_all(b"WAVEfmt ")?;
        w.write_all(&16u32.to_le_bytes())?;
        w.write_all(&1u16.to_le_bytes())?; // PCM
        w.write_all(&1u16.to_le_bytes())?; // mono
        w.write_all(&self.sample_rate.to_le_bytes())?;
        w.write_all(&byte_rate.to_le_bytes())?;
        w.write_all(&2u16.to_le_bytes())?;
        w.write_all(&16u16.to_le_bytes())?;
        w.write_all(b"data")?;
        w.write_all(&(data_len as u32).to_le_bytes())?;
        for &s in &self.samples {
            let v = (s.clamp(-1.0, 1.0) * 32_767.0).round() as i16;
            w.write_all(&v.to_le_bytes())?;
        }
        w.flush()?;
        Ok(())
    }
}

pub fn mix_buffers(a: &WavBuffer, b: &WavBuffer) -> WavBuffer {
    let sample_rate = a.sample_rate;
    let len = a.samples.len().max(b.samples.len());
    let mut out = vec![0.0; len];
    for (i, s) in a.samples.iter().enumerate() {
        out[i] += *s;
    }
    for (i, s) in b.samples.iter().enumerate() {
        if i < out.len() {
            out[i] = (out[i] + *s).clamp(-1.0, 1.0);
        }
    }
    WavBuffer {
        samples: out,
        sample_rate,
    }
}
