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
/// least `min_blocks` by more than `delta` with one of them past `noise`.
///
/// The codec floor: on a still picture, H.264 drifts across P-frames and a
/// keyframe re-quantizes it, so a few dozen block means move by up to ~6
/// levels, scattered, never past the quantizer's error (*pitagoras*, a
/// still closing card: 27 blocks over 4 at f2499, 43 at the keyframe f2500,
/// the largest 5.8). One comparison cannot tell that from a slow fade
/// moving 43 blocks by 5. Time can: `b` is the last frame that moved, so
/// real motion keeps adding up against it and passes `noise` a frame or
/// two later, while the codec's error stays bounded however long the card
/// holds. The floor costs a slow change a frame or two of lag, never its
/// detection.
pub fn moved(a: &[f32], b: &[f32], spec: &super::profile::MotionSpec) -> bool {
    let (mut n, mut loud) = (0, false);
    for (x, y) in a.iter().zip(b) {
        let d = (x - y).abs();
        if d > spec.strong {
            return true;
        }
        if d > spec.delta {
            n += 1;
            loud |= d > spec.noise;
            if n >= spec.min_blocks && loud {
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
    use super::super::profile::MotionSpec;
    use super::*;

    #[test]
    fn covered_is_the_overlap() {
        assert_eq!(covered(1.0, 3.0, &[(0.0, 1.5), (2.5, 9.0)]), 1.0);
        assert_eq!(covered(1.0, 3.0, &[(4.0, 5.0)]), 0.0);
    }

    fn spec() -> MotionSpec {
        MotionSpec {
            max_still_with_voice: 4.0,
            closing_hold: 1.5,
            strong: 24.0,
            delta: 4.0,
            min_blocks: 30,
            noise: 8.0,
        }
    }

    /// The rule before the codec floor: any block over `delta` counted.
    fn without_floor() -> MotionSpec {
        MotionSpec {
            noise: 4.0,
            ..spec()
        }
    }

    #[test]
    fn moved_is_one_strong_block_or_many_weak_ones() {
        let spec = MotionSpec {
            min_blocks: 3,
            ..spec()
        };
        let a = vec![0f32; 100];
        let mut b = a.clone();
        b[0] = 30.0;
        assert!(moved(&a, &b, &spec));
        b[0] = 9.0;
        b[1] = 7.0;
        assert!(!moved(&a, &b, &spec), "two weak blocks are codec");
        b[2] = 7.0;
        assert!(moved(&a, &b, &spec));
        b[0] = 7.0;
        assert!(
            !moved(&a, &b, &spec),
            "weak blocks none past the floor are codec"
        );
    }

    const W: usize = 320;
    const H: usize = 240;
    const PAPER: u8 = 236;
    const INK: u8 = 40;

    /// A deterministic xorshift, for noise.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        /// Uniform in −m..=m.
        fn pm(&mut self, m: i32) -> i32 {
            (self.next() % (2 * m as u64 + 1)) as i32 - m
        }
    }

    fn frame(paint: impl Fn(usize, usize) -> u8) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(W * H * 3);
        for y in 0..H {
            for x in 0..W {
                let v = paint(x, y);
                rgb.extend([v, v, v]);
            }
        }
        rgb
    }

    /// A still closing card: a disc (the medallion) and a line of text.
    fn card(x: usize, y: usize) -> u8 {
        let (dx, dy) = (x as f32 - 160.0, y as f32 - 100.0);
        let disc = dx * dx + dy * dy < 60.0 * 60.0;
        let text = (40..280).contains(&x) && (190..206).contains(&y) && x % 5 < 3;
        if disc || text {
            INK
        } else {
            PAPER
        }
    }

    /// `moving` for a run of frames, the way the gate scans: each frame
    /// against the last one that moved.
    fn scan(frames: impl IntoIterator<Item = Vec<u8>>, spec: &MotionSpec) -> Vec<bool> {
        let (mut blocks, mut anchor, mut moving) = (Vec::new(), Vec::new(), Vec::new());
        for (i, rgb) in frames.into_iter().enumerate() {
            block_means(&rgb, W, H, (0, 0), &mut blocks);
            let m = i == 0 || moved(&blocks, &anchor, spec);
            moving.push(m);
            if m {
                std::mem::swap(&mut blocks, &mut anchor);
            }
        }
        moving
    }

    /// The longest run of frames after the first that did not move.
    fn longest_still(moving: &[bool]) -> usize {
        let (mut run, mut worst) = (0, 0);
        for &m in &moving[1..] {
            run = if m { 0 } else { run + 1 };
            worst = worst.max(run);
        }
        worst
    }

    #[test]
    fn codec_noise_on_a_still_card_is_not_motion() {
        // Quantization-like noise, diffuse and up to ±6 against the clean
        // card: each 4 × 4 transform block's DC shifts (that moves the 8 × 8
        // mean), each pixel jitters.
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut noisy = || {
            let dc: Vec<i32> = (0..(W / 4) * (H / 4)).map(|_| rng.pm(6)).collect();
            let px: Vec<i32> = (0..W * H).map(|_| rng.pm(6)).collect();
            frame(|x, y| {
                let v = i32::from(card(x, y)) + dc[(y / 4) * (W / 4) + x / 4] + px[y * W + x];
                v.clamp(0, 255) as u8
            })
        };
        let (a, b) = (frame(card), noisy());
        let (mut ma, mut mb) = (Vec::new(), Vec::new());
        block_means(&a, W, H, (0, 0), &mut ma);
        block_means(&b, W, H, (0, 0), &mut mb);
        let over: Vec<f32> = ma
            .iter()
            .zip(&mb)
            .map(|(x, y)| (x - y).abs())
            .filter(|&d| d > 4.0)
            .collect();
        assert!(
            over.len() >= 30,
            "the noise is diffuse: {} blocks over 4",
            over.len()
        );
        assert!(over.iter().all(|&d| d < 8.0));
        assert!(
            moved(&ma, &mb, &without_floor()),
            "the old rule read it as motion"
        );
        assert!(!moved(&ma, &mb, &spec()));
        // and over a whole hold: one still stretch from the first frame
        let moving = scan(
            std::iter::once(frame(card)).chain((0..60).map(|_| noisy())),
            &spec(),
        );
        assert_eq!(moving.iter().filter(|&&m| m).count(), 1, "{moving:?}");
    }

    #[test]
    fn keyframe_jump_on_the_closing_card_is_not_motion() {
        // *pitagoras* at 2504 frames: against the frame where the medallion
        // finished, P-frame drift left 27 blocks over 4 at f2499; the
        // keyframe at f2500 re-quantized the card, 43 blocks over 4, the
        // largest 5.80, and the old rule ended the closing hold there.
        let anchor = vec![120f32; (W / BLOCK) * (H / BLOCK)];
        let shifted = |n: usize, top: f32| {
            let mut b = anchor.clone();
            for (k, v) in b.iter_mut().take(n).enumerate() {
                *v += if k % 2 == 0 { 4.5 } else { -4.5 };
            }
            b[n / 2] = 120.0 + top;
            b
        };
        let (drift, key) = (shifted(27, 5.27), shifted(43, 5.80));
        assert!(!moved(&drift, &anchor, &spec()));
        assert!(moved(&key, &anchor, &without_floor()), "the false change");
        assert!(!moved(&key, &anchor, &spec()));
    }

    #[test]
    fn slow_text_fade_still_moves() {
        // a 200 × 60 px text block (strokes on 3 px of every 5) fading in,
        // 3 % per frame, off the block grid
        let text = |t: f32| {
            frame(move |x, y| {
                let inside = (61..261).contains(&x) && (93..153).contains(&y) && x % 5 < 3;
                if inside {
                    (f32::from(PAPER) + t * (f32::from(INK) - f32::from(PAPER))).round() as u8
                } else {
                    PAPER
                }
            })
        };
        let moving = scan((0..=34).map(|k| text((0.03 * k as f32).min(1.0))), &spec());
        assert!(moving.iter().filter(|&&m| m).count() >= 10, "{moving:?}");
        assert!(longest_still(&moving) <= 2, "{moving:?}");
        assert!(
            moving[30..].contains(&true),
            "still moving at the end of the fade"
        );
    }

    #[test]
    fn small_object_moving_slowly_still_moves() {
        // a 20 px square, 2 px per frame, in ink and in a faint tint
        for (tint, gap) in [(INK, 0), (PAPER - 30, 3)] {
            let at = |k: usize| {
                frame(move |x, y| {
                    let x0 = 37 + 2 * k;
                    if (x0..x0 + 20).contains(&x) && (101..121).contains(&y) {
                        tint
                    } else {
                        PAPER
                    }
                })
            };
            let moving = scan((0..40).map(at), &spec());
            assert!(longest_still(&moving) <= gap, "tint {tint}: {moving:?}");
        }
    }
}
