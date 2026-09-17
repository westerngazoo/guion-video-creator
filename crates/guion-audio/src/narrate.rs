use std::path::{Path, PathBuf};

use guion_core::Screenplay;

use crate::bed::timeline_duration;
use crate::engine::{engine_by_name, NarrationEngine};
use crate::error::AudioError;
use crate::script::{collect_cues, Cue};
use crate::wav::{SAMPLE_RATE, WavBuffer};

pub struct NarrateOptions {
    pub engine: String,
    pub piper_model: Option<String>,
    pub script_path: Option<PathBuf>,
    pub include_bed: bool,
}

impl Default for NarrateOptions {
    fn default() -> Self {
        NarrateOptions {
            engine: "scaffold".into(),
            piper_model: None,
            script_path: None,
            include_bed: false,
        }
    }
}

/// Build a timeline-aligned narration WAV for a screenplay.
pub fn narrate_screenplay(
    sp: &Screenplay,
    screenplay_dir: &Path,
    opts: &NarrateOptions,
) -> Result<WavBuffer, AudioError> {
    let script = opts
        .script_path
        .clone()
        .or_else(|| resolve_narration_script(sp, screenplay_dir));
    let cues = collect_cues(sp, script.as_deref())?;
    let engine = engine_by_name(&opts.engine, opts.piper_model.as_deref())?;
    synthesize_timeline(sp, &cues, engine.as_ref(), opts.include_bed)
}

pub fn default_narration_path(slug: &str) -> PathBuf {
    PathBuf::from("out").join(slug).join("narration.wav")
}

pub fn write_narration(
    sp: &Screenplay,
    screenplay_dir: &Path,
    out: &Path,
    opts: &NarrateOptions,
) -> Result<PathBuf, AudioError> {
    let wav = narrate_screenplay(sp, screenplay_dir, opts)?;
    wav.write_wav(out)?;
    Ok(out.to_path_buf())
}

fn synthesize_timeline(
    sp: &Screenplay,
    cues: &[Cue],
    engine: &dyn NarrationEngine,
    include_bed: bool,
) -> Result<WavBuffer, AudioError> {
    let total = timeline_duration(sp);
    let mut track = WavBuffer::silence(total, SAMPLE_RATE);

    for cue in cues {
        let speech = engine.synthesize(&cue.text)?;
        let window = (cue.at.end - cue.at.start).max(0.05);
        let placed = fit_to_window(&speech, window);
        track.mix_at(cue.at.start, &placed);
    }

    track.apply_fade(220);

    if include_bed {
        let gen = sp
            .audio
            .as_ref()
            .and_then(|a| a.generator.as_deref())
            .unwrap_or("fbf-default");
        let bed = crate::bed::generate_bed(gen, total, sp.meta.fps)?;
        Ok(crate::wav::mix_buffers(&bed, &track))
    } else {
        Ok(track)
    }
}

/// Time-stretch or trim speech to fit the cue window.
pub fn fit_to_window(samples: &[f32], window_secs: f64) -> Vec<f32> {
    let target = (window_secs * SAMPLE_RATE as f64).ceil() as usize;
    if samples.is_empty() {
        return vec![0.0; target];
    }
    if samples.len() == target {
        return samples.to_vec();
    }
    if samples.len() < target {
        let mut out = samples.to_vec();
        out.resize(target, 0.0);
        return out;
    }
    // Linear resample down to target length.
    let mut out = Vec::with_capacity(target);
    for i in 0..target {
        let src = (i as f64 / target as f64 * samples.len() as f64) as usize;
        out.push(samples[src.min(samples.len() - 1)]);
    }
    out
}

pub fn resolve_narration_script(sp: &Screenplay, screenplay_dir: &Path) -> Option<PathBuf> {
    sp.audio
        .as_ref()
        .and_then(|a| a.narration_script.as_deref())
        .map(|rel| resolve_relative(screenplay_dir, rel))
}

pub fn resolve_relative(base: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let candidate = base.join(p);
    if candidate.exists() {
        return candidate;
    }
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ws = workspace.join(p);
    if ws.exists() {
        return ws;
    }
    candidate
}

pub fn resolve_audio_path(sp: &Screenplay, screenplay_dir: &Path, slug: &str) -> Option<PathBuf> {
    if let Some(rel) = sp.audio.as_ref().and_then(|a| a.path.as_deref()) {
        let p = resolve_relative(screenplay_dir, rel);
        if p.exists() {
            return Some(p);
        }
    }
    let narr = default_narration_path(slug);
    if narr.exists() {
        return Some(narr);
    }
    None
}

/// Final mux audio: narration + optional bed + explicit path.
pub fn mix_for_encode(
    sp: &Screenplay,
    screenplay_dir: &Path,
    narration_wav: Option<&Path>,
) -> Result<Option<PathBuf>, AudioError> {
    let slug = &sp.meta.slug;
    let mut layers: Vec<WavBuffer> = Vec::new();

    if let Some(path) = narration_wav {
        if path.exists() {
            layers.push(read_wav_file(path)?);
        }
    } else if let Some(path) = resolve_audio_path(sp, screenplay_dir, slug) {
        layers.push(read_wav_file(&path)?);
    }

    if let Some(gen) = sp.audio.as_ref().and_then(|a| a.generator.as_deref()) {
        let total = timeline_duration(sp);
        let bed = crate::bed::generate_bed(gen, total, sp.meta.fps)?;
        layers.insert(0, bed);
    }

    if layers.is_empty() {
        return Ok(None);
    }

    let mut mixed = layers[0].clone();
    for layer in layers.iter().skip(1) {
        mixed = crate::wav::mix_buffers(&mixed, layer);
    }
    mixed.apply_fade(220);
    let out = PathBuf::from("out").join(slug).join("mixed.wav");
    mixed.write_wav(&out)?;
    Ok(Some(out))
}

fn read_wav_file(path: &Path) -> Result<WavBuffer, AudioError> {
    let data = std::fs::read(path)?;
    if data.len() < 44 {
        return Err(AudioError::Engine(format!("wav demasiado corto: {}", path.display())));
    }
    let sample_rate = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    let channels = u16::from_le_bytes([data[22], data[23]]) as usize;
    let bits = u16::from_le_bytes([data[34], data[35]]);
    let pcm = &data[44..];
    let mut mono = Vec::new();
    if bits == 16 {
        for chunk in pcm.chunks(2 * channels) {
            if chunk.len() >= 2 {
                mono.push(i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32_768.0);
            }
        }
    } else {
        return Err(AudioError::Engine("solo wav PCM 16-bit".into()));
    }
    Ok(WavBuffer {
        samples: mono,
        sample_rate,
    })
}
