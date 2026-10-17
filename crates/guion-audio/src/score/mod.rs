//! A reel's sound design as data (R-0011): `sonido.toml` + `eventos.json`
//! (+ `voz.wav` and `voz.json`) → `audio.wav`.
//!
//! The engine reads the score, resolves every time against the scene's
//! events, places each sound on its bus, gives the buses one room, ducks the
//! bed under the voice and mixes. It is built on [`crate::synth`] (R-0008).
//! **What the sounds are is the caller's:** a brand registers its
//! instruments by name in [`Instruments`]; no instrument lives here.
//!
//! **One clock.** No time is written by hand: the score names an event of
//! `eventos.json` (the scene writes it), with an offset if needed: `"qed"`,
//! `"qed - 1.2"`, `"q2-fin + 0.5"`, `"fin - 0.5"`, `"0"`, or a fraction of a
//! span: `"m1..m1-fin @ 0.69"`. An exact name wins (`"menos-1"` is the event
//! «menos-1»); to offset, the sign goes between spaces.
//!
//! **Deterministic.** Each sound takes its noise from its own stream,
//! `seed_for(score seed, n)`; `n` is its number (or its `semilla`), so moving
//! one hit does not change the noise of the others.
//!
//! The format (keys in Spanish, as the scores are written):
//!
//! ```toml
//! [sonido]
//! semilla = 1
//! sala = { t60 = 2.8, largo = 3.4, oscuro = 4200.0, envio = 0.8 }
//!
//! [voz]                # optional: voz.wav (48 kHz, mono) up front, centred
//! paso_alto = 70.0
//! nivel = 0.707
//! sala = { t60 = 1.2, nivel = 0.06 }
//!
//! [mezcla]
//! cama = 0.30          # the bed's peak against the voice's (1)
//! # cama_rms = -11.2   # or, better: the bed's RMS, in dB under the voice's
//! pico = 0.89
//! entrada = 0.02       # fades, s
//! salida = 0.5
//!
//! [agachar]            # the bed, under each line of voz.json
//! por = "lineas"       # or "seguidor" (the level of voz.wav; ataque, suelta, umbral)
//! antes = 0.08
//! despues = 0.12
//! rampa = 0.08
//! unir = 0.5           # optional: two lines less than 0.5 s apart are one window
//! buses = { dron = -10.0, golpe = -5.0 }
//!
//! [[dron]]             # bus "dron" unless `bus` says otherwise
//! instrumento = "…"    # a Kind::Drone the caller registered (or its default)
//! ganancia = 0.16
//! sala = 0.25
//! lfo = { hz = 0.06, hondo = 0.12 }
//! envolvente = [["0", 0.4], ["qed", 0.7], ["fin - 0.5", 0.6], ["fin", 0.0]]
//!
//! [[seguidor]]         # a series of eventos.json moves a tone (envolvente, lfo as the drone)
//! serie = "vueltas"
//! instrumento = "…"    # a Kind::Follower
//! hasta = "q2-fin + 0.5"
//!
//! [[golpe]]            # bus "golpe"
//! en = "qed"           # or a list: ["problema", "planum"]
//! instrumento = "…"    # a Kind::Hit
//! ganancia = 1.15
//! sala = 0.55
//! ```
//!
//! **Hits that repeat.** A hit with several instants (a list in `en`, or
//! `repetir`) takes for any number a list with one value per hit, and
//! `paso`, which adds `i ·` step to hit `i` (the first is 0). `repetir` goes
//! with `n` and `hasta` (n hits spread from `desde` to before `hasta`), with
//! `cada` (or `cada_cuadros`) and `hasta`, or with `cada` and `n`. Up to 100
//! hits per table. `envolvente` on a hit scales each hit's gain by its value
//! at that instant; `hasta` gives the hit a length (`Params::duration`).
//!
//! Common keys of every table: `ganancia`, `pan`, `pan_hasta`, `sala`, `bus`,
//! `semilla`. A key not read by the engine or the instrument is an error
//! naming the table. If anything fails, nothing is written.

mod instrument;
mod mix;
mod params;
mod parse;
mod timeline;

use std::path::{Path, PathBuf};

pub use instrument::{
    sound, sound_lasting, Builder, Ctx, FnSound, Instruments, Kind, Rendered, Sound,
};
pub use mix::{hits_tsv, merge_lines, render, write_f32, Output, Voice};
pub use params::Params;
pub use parse::{
    BedLevel, DuckBy, DuckCfg, MixCfg, Piece, RoomCfg, Score, Source, VoiceCfg, MAX_HITS,
};
pub use timeline::{Series, Timeline};

use crate::synth::{from_pcm16, to_pcm16};
use crate::{read_wav_pcm16, write_wav, AudioError};

/// The sample rate of every score, Hz.
pub const RATE: f64 = 48_000.0;

/// The `(t0, t1)` of each line of a `voz.json` (R-0007's narration
/// timeline), in order.
pub fn voice_lines_from_json(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let j: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("voz.json: {e}"))?;
    let lines = j
        .get("lineas")
        .and_then(serde_json::Value::as_array)
        .ok_or("voz.json: falta lineas")?;
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let num = |k: &str| {
                l.get(k)
                    .and_then(serde_json::Value::as_f64)
                    .ok_or(format!("voz.json: la línea {} no tiene {k}", i + 1))
            };
            Ok((num("t0")?, num("t1")?))
        })
        .collect()
}

/// Where [`render_post`] writes, and what it reports.
#[derive(Debug)]
pub struct PostOutput {
    /// The `audio.wav` written.
    pub path: PathBuf,
    /// The reel's length, s (from `eventos.json`).
    pub duration: f64,
    /// How the bed sits under the voice (see [`Output::report`]).
    pub report: String,
}

fn located(p: &Path) -> impl Fn(String) -> AudioError + '_ {
    move |e| AudioError::InvalidScore(format!("{}: {e}", p.display()))
}

fn read(path: &Path) -> Result<String, AudioError> {
    std::fs::read_to_string(path)
        .map_err(|e| AudioError::InvalidScore(format!("{}: {e}", path.display())))
}

/// A post folder → its `audio.wav` (48 kHz, stereo, 16 bit): reads
/// `eventos.json` and `sonido.toml`, and with `[voz]` also `voz.wav` (48 kHz
/// mono) and `voz.json`. `out` defaults to `audio.wav` in the folder.
/// With `stems`, also writes there the voice, the bed and each bus dry
/// (`<bus>-seco`) and with its room as interleaved `f32`, and `golpes.tsv`.
/// If anything fails, nothing is written.
pub fn render_post(
    dir: &Path,
    instruments: &Instruments,
    out: Option<&Path>,
    stems: Option<&Path>,
) -> Result<PostOutput, AudioError> {
    let ev = dir.join("eventos.json");
    let times = Timeline::from_json(&read(&ev)?).map_err(located(&ev))?;
    let toml_path = dir.join("sonido.toml");
    let score =
        Score::parse(&read(&toml_path)?, &times, instruments).map_err(located(&toml_path))?;

    let (samples, lines) = if score.voice.is_some() {
        let wav = dir.join("voz.wav");
        let bytes = std::fs::read(&wav).map_err(|e| {
            AudioError::InvalidScore(format!("{}: {e} (falta la narración)", wav.display()))
        })?;
        let pcm = read_wav_pcm16(&bytes).map_err(|e| located(&wav)(e.to_string()))?;
        if pcm.sample_rate != 48_000 || pcm.channels != 1 {
            return Err(located(&wav)(format!(
                "se esperaba 48 kHz mono; es {} Hz, {} canales",
                pcm.sample_rate, pcm.channels
            )));
        }
        let vj = dir.join("voz.json");
        let lines = voice_lines_from_json(&read(&vj)?).map_err(located(&vj))?;
        (from_pcm16(&pcm.samples), lines)
    } else {
        (Vec::new(), Vec::new())
    };
    let voice = score.voice.map(|_| Voice {
        samples: &samples,
        lines: &lines,
    });
    let s = render(&score, &times, voice).map_err(AudioError::InvalidScore)?;

    let path = out.map_or_else(|| dir.join("audio.wav"), Path::to_path_buf);
    if let Some(stems) = stems {
        std::fs::create_dir_all(stems)?;
        let mut all = vec![("voz".to_string(), &s.voice), ("cama".to_string(), &s.bed)];
        all.extend(s.dry.iter().map(|(b, x)| (format!("{b}-seco"), x)));
        all.extend(s.buses.iter().map(|(b, x)| (b.clone(), x)));
        std::fs::write(stems.join("golpes.tsv"), hits_tsv(&score))?;
        for (name, x) in all {
            write_f32(&stems.join(format!("{name}.f32")), x)?;
        }
    }
    write_wav(&path, &to_pcm16(&s.mix, 48_000))?;
    Ok(PostOutput {
        path,
        duration: times.duration,
        report: s.report,
    })
}
