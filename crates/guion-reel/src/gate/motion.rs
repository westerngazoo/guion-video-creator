//! The motion scan (R-0012 AC5): which frames change, the still stretches
//! while the voice speaks, and the closing hold.

use super::profile::Profile;
use super::report::{at_frame, Finding, MAX_AT};

/// The side of the blocks motion is measured on, px.
pub const BLOCK: usize = 8;

/// The mean luma (BT.709 weights) of each 8 × 8 block, skipping the blocks
/// that touch the rows in `skip` (the subtitle band: a subtitle changing is
/// not the scene moving). Averaging first is what keeps the codec out: a
/// keyframe re-quantizes sharp edges by tens of levels per pixel, but the
/// block's mean barely moves, while anything that really moves changes a
/// few blocks a lot or many blocks a little.
pub fn block_means(rgb: &[u8], w: usize, h: usize, skip: (usize, usize), out: &mut Vec<f32>) {
    out.clear();
    let (bw, bh) = (w / BLOCK, h / BLOCK);
    let mut row = vec![0.0f32; bw];
    for by in 0..bh {
        let (y0, y1) = (by * BLOCK, by * BLOCK + BLOCK);
        if y1 > skip.0 && y0 < skip.1 {
            continue;
        }
        row.iter_mut().for_each(|v| *v = 0.0);
        for y in y0..y1 {
            let line = &rgb[y * w * 3..y * w * 3 + bw * BLOCK * 3];
            for (x, p) in line.as_chunks::<3>().0.iter().enumerate() {
                row[x / BLOCK] +=
                    0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2]);
            }
        }
        out.extend(row.iter().map(|v| v / (BLOCK * BLOCK) as f32));
    }
}

/// Whether two block maps differ: one block by more than `strong`, or at
/// least `min_blocks` by more than `delta`.
pub fn moved(a: &[f32], b: &[f32], spec: &super::profile::MotionSpec) -> bool {
    let mut n = 0;
    for (x, y) in a.iter().zip(b) {
        let d = (x - y).abs();
        if d > spec.strong {
            return true;
        }
        if d > spec.delta {
            n += 1;
            if n >= spec.min_blocks {
                return true;
            }
        }
    }
    false
}

/// Seconds of `[a, b)` covered by the union of `spans`.
fn covered(a: f64, b: f64, spans: &[(f64, f64)]) -> f64 {
    spans
        .iter()
        .map(|&(s0, s1)| (b.min(s1) - a.max(s0)).max(0.0))
        .sum()
}

/// The checks on `moving[i]` (frame i differs from the last frame that moved).
pub fn check(moving: &[bool], voice: &[(f64, f64)], p: &Profile) -> Vec<Finding> {
    let fps = p.canvas.fps;
    let n = moving.len();
    let mut still = Vec::new();
    let mut worst: f64 = 0.0;
    let mut i = 1;
    while i < n {
        if moving[i] {
            i += 1;
            continue;
        }
        // frames i−1 … j−1 show the same picture
        let mut j = i;
        while j < n && !moving[j] {
            j += 1;
        }
        let (a, b) = ((i - 1) as f64 / fps, j as f64 / fps);
        let spoken = covered(a, b, voice);
        // the closing hold counts too: a last line said over a frozen frame is still a frozen frame
        worst = worst.max(spoken);
        if spoken > p.motion.max_still_with_voice {
            still.push((
                spoken,
                format!(
                    "{:.2}–{:.2} s (f{}–f{}), {spoken:.1} s with voice",
                    a,
                    b,
                    i - 1,
                    j - 1
                ),
            ));
        }
        i = j;
    }
    still.sort_by(|x, y| y.0.total_cmp(&x.0));
    let still: Vec<String> = still.into_iter().take(MAX_AT).map(|x| x.1).collect();
    let last = (1..n).rev().find(|&i| moving[i]).unwrap_or(0);
    let hold = (n - 1 - last) as f64 / fps;
    vec![
        Finding::from_misses(
            "still-voice",
            format!(
                "longest still stretch under the voice {worst:.1} s ≤ {:.1}",
                p.motion.max_still_with_voice
            ),
            format!(
                "{} still stretches over {:.1} s while the voice speaks",
                still.len(),
                p.motion.max_still_with_voice
            ),
            still,
        ),
        if hold >= p.motion.closing_hold {
            Finding::pass(
                "closing-hold",
                format!(
                    "{hold:.2} s still at the end ≥ {:.1}",
                    p.motion.closing_hold
                ),
            )
        } else {
            Finding::fail(
                "closing-hold",
                format!(
                    "{hold:.2} s still at the end < {:.1}",
                    p.motion.closing_hold
                ),
                vec![format!("last change at {}", at_frame(last, fps))],
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covered_is_the_overlap() {
        assert_eq!(covered(1.0, 3.0, &[(0.0, 1.5), (2.5, 9.0)]), 1.0);
        assert_eq!(covered(1.0, 3.0, &[(4.0, 5.0)]), 0.0);
    }

    #[test]
    fn moved_is_one_strong_block_or_many_weak_ones() {
        let spec = super::super::profile::MotionSpec {
            max_still_with_voice: 4.0,
            closing_hold: 1.5,
            strong: 24.0,
            delta: 4.0,
            min_blocks: 3,
        };
        let a = vec![0f32; 100];
        let mut b = a.clone();
        b[0] = 30.0;
        assert!(moved(&a, &b, &spec));
        b[0] = 7.0;
        b[1] = 7.0;
        assert!(!moved(&a, &b, &spec), "two weak blocks are codec");
        b[2] = 7.0;
        assert!(moved(&a, &b, &spec));
    }
}
