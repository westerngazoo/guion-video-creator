//! The audio checks (R-0012 AC6): the platform's loudness, measured with
//! guion-encode's EBU R128 reader (the same one the mix converges with), and
//! the bed in the short gaps between lines.
//!
//! **The bed in a short gap.** Under a line the bed is ducked; if it comes
//! back up in a gap shorter than `audio.short_gap`, the reel "pumps". The
//! voice is silent in the gap, so the mix there is the bed: its RMS, relative
//! to the RMS of the two lines around it (where the voice dominates), must
//! stay at or under `audio.gap_max_db`. A bed that stays ducked through the
//! gap sits about as far under the voice as the brand's mix puts it; one
//! that comes back up sits much closer.

use std::path::Path;

use super::profile::Profile;
use super::report::{Finding, MAX_AT};
use super::GateError;
use crate::narration::Narration;

const RATE: u32 = 48_000;
/// What is left out at each edge of a gap (the voice's last and first breath), s.
const EDGE: f64 = 0.03;

fn rms_db(x: &[f32]) -> f64 {
    if x.is_empty() {
        return f64::NEG_INFINITY;
    }
    let e: f64 = x.iter().map(|&s| f64::from(s) * f64::from(s)).sum::<f64>() / x.len() as f64;
    10.0 * e.max(1e-20).log10()
}

fn slice(x: &[f32], a: f64, b: f64) -> &[f32] {
    let i = ((a * f64::from(RATE)).round().max(0.0) as usize).min(x.len());
    let j = ((b * f64::from(RATE)).round().max(0.0) as usize).clamp(i, x.len());
    &x[i..j]
}

/// The loudness finding.
pub fn loudness(mp4: &Path, p: &Profile) -> Result<Finding, GateError> {
    let m = guion_encode::measure(mp4).map_err(|e| GateError::Input(e.to_string()))?;
    let a = &p.audio;
    let detail = format!(
        "{:.1} LUFS (target {:.1} ± {:.1}), true peak {:.1} dBTP (≤ {:.1})",
        m.integrated_lufs, a.lufs, a.tolerance, m.true_peak_dbtp, a.true_peak
    );
    let ok = (m.integrated_lufs - a.lufs).abs() <= a.tolerance && m.true_peak_dbtp <= a.true_peak;
    Ok(if ok {
        Finding::pass("loudness", detail)
    } else {
        Finding::fail("loudness", detail, Vec::new())
    })
}

/// The levels of the short gaps, relative to their lines: `(t1, gap, dB)`.
/// A gap that holds one of `exempt` (beat changes) is left out.
pub fn gap_levels(
    samples: &[f32],
    narration: &Narration,
    short_gap: f64,
    exempt: &[f64],
) -> Vec<(f64, f64, f64)> {
    let mut out = Vec::new();
    for w in narration.lines.windows(2) {
        let gap = w[1].t0 - w[0].t1;
        if gap <= 2.0 * EDGE || gap >= short_gap {
            continue;
        }
        if exempt.iter().any(|&t| t >= w[0].t1 && t <= w[1].t0) {
            continue;
        }
        let bed = rms_db(slice(samples, w[0].t1 + EDGE, w[1].t0 - EDGE));
        let voice = 0.5
            * (rms_db(slice(samples, w[0].t0, w[0].t1)) + rms_db(slice(samples, w[1].t0, w[1].t1)));
        out.push((w[0].t1, gap, bed - voice));
    }
    out
}

/// The pumping finding.
pub fn pumping(
    mp4: &Path,
    narration: &Narration,
    events: &[(String, f64)],
    p: &Profile,
) -> Result<Finding, GateError> {
    let x = super::video::audio_mono(mp4, RATE)?;
    let beats: Vec<f64> = events
        .iter()
        .filter(|(k, _)| {
            p.audio
                .beat_events
                .iter()
                .any(|b| k.starts_with(b.as_str()))
        })
        .map(|(_, t)| *t)
        .collect();
    let gaps = gap_levels(&x, narration, p.audio.short_gap, &beats);
    let worst = gaps.iter().map(|g| g.2).fold(f64::NEG_INFINITY, f64::max);
    let mut bad: Vec<&(f64, f64, f64)> = gaps.iter().filter(|g| g.2 > p.audio.gap_max_db).collect();
    bad.sort_by(|a, b| b.2.total_cmp(&a.2));
    let at = bad
        .iter()
        .take(MAX_AT)
        .map(|g| format!("{:.2} s gap {:.2} s at {:+.1} dB", g.0, g.1, g.2))
        .collect();
    let n = gaps.len();
    Ok(if n == 0 {
        Finding::pass(
            "bed-gaps",
            format!(
                "no gaps under {:.2} s outside beat changes",
                p.audio.short_gap
            ),
        )
    } else {
        Finding::from_misses(
            "bed-gaps",
            format!("{n} gaps under {:.2} s (beat changes aside), loudest {worst:+.1} dB vs the voice (≤ {:+.1})", p.audio.short_gap, p.audio.gap_max_db),
            format!("{} of {n} short gaps let the bed up (> {:+.1} dB vs the voice)", bad.len(), p.audio.gap_max_db),
            at,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loud_gap_is_measured_against_its_lines() {
        let line = |id: &str, t0: f64, t1: f64| {
            format!(
                r#"{{"id":"{id}","texto":"A.","subtitulo":"A.","t0":{t0},"t1":{t1},"frases":[{{"texto":"A.","t0":{t0},"t1":{t1}}}]}}"#
            )
        };
        let json = format!(
            r#"{{"duracion_voz":3.0,"lineas":[{},{}]}}"#,
            line("a", 0.0, 1.0),
            line("b", 1.3, 2.3)
        );
        let n = Narration::from_json(&json).unwrap();
        // voice at amplitude 0.5, the gap at 0.05: 20 dB under
        let mut x: Vec<f32> = (0..(2.5 * RATE as f64) as usize)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        for s in &mut x[(1.0 * RATE as f64) as usize..(1.3 * RATE as f64) as usize] {
            *s *= 0.1;
        }
        let g = gap_levels(&x, &n, 0.5, &[]);
        assert!(
            gap_levels(&x, &n, 0.5, &[1.1]).is_empty(),
            "a beat change is not pumping"
        );
        assert_eq!(g.len(), 1);
        assert!((g[0].2 + 20.0).abs() < 0.01, "{g:?}");
    }
}
