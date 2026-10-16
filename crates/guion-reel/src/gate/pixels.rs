//! The per-frame pixel scan (R-0012 AC4): which token each decoded pixel is,
//! the regions of a role, and the zones that must stay empty.
//!
//! Approximate by design. A decoded pixel is a token if it lies within
//! `scan.tolerance` of it (H.264 and 4:2:0 move colours a little, and
//! anti-aliased edges are blends that match nothing, so they are ignored).
//! A *region* is a set of a role's pixels where each lies within
//! `scan.merge` px of another (glyphs of one formula join; two formulas
//! apart do not), counted only if it has `scan.min_area` pixels or more.

use super::profile::{Profile, Rgb};

/// "Matches no token."
pub const NONE: u8 = u8::MAX;

/// A 16 M-entry table: RGB → index of the nearest token within tolerance.
pub struct Classifier {
    lut: Vec<u8>,
    pub names: Vec<String>,
}

impl Classifier {
    pub fn new(profile: &Profile) -> Classifier {
        let toks: Vec<(String, Rgb)> = profile
            .tokens
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        assert!(toks.len() < NONE as usize, "at most 254 tokens");
        let tol2 = profile.scan.tolerance * profile.scan.tolerance;
        let mut lut = vec![NONE; 1 << 24];
        // only the cubes around each token can match: fill those, nearest wins
        let r = profile.scan.tolerance.clamp(0.0, 147.0).floor() as i32;
        // squared distances fit in u16 for a tolerance up to 255
        let mut best = vec![u16::MAX; 1 << 24];
        for (k, (_, c)) in toks.iter().enumerate() {
            let [cr, cg, cb] = c.map(i32::from);
            for dr in -r..=r {
                for dg in -r..=r {
                    for db in -r..=r {
                        let d2 = dr * dr + dg * dg + db * db;
                        let (x, y, z) = (cr + dr, cg + dg, cb + db);
                        if f64::from(d2) > tol2
                            || !(0..256).contains(&x)
                            || !(0..256).contains(&y)
                            || !(0..256).contains(&z)
                        {
                            continue;
                        }
                        let i = ((x as usize) << 16) | ((y as usize) << 8) | z as usize;
                        if (d2 as u16) < best[i] {
                            best[i] = d2 as u16;
                            lut[i] = k as u8;
                        }
                    }
                }
            }
        }
        Classifier {
            lut,
            names: toks.into_iter().map(|(k, _)| k).collect(),
        }
    }

    /// The indices of the named tokens.
    pub fn ids(&self, names: &[String]) -> Vec<u8> {
        names
            .iter()
            .filter_map(|n| self.names.iter().position(|m| m == n).map(|i| i as u8))
            .collect()
    }

    /// One class per pixel of an RGB24 frame.
    pub fn classify(&self, rgb: &[u8], out: &mut Vec<u8>) {
        out.clear();
        out.extend(
            rgb.as_chunks::<3>().0.iter().map(|p| {
                self.lut[((p[0] as usize) << 16) | ((p[1] as usize) << 8) | p[2] as usize]
            }),
        );
    }
}

/// A set of token ids as a 256-entry membership table.
pub fn set(ids: &[u8]) -> [bool; 256] {
    let mut s = [false; 256];
    for &i in ids {
        s[i as usize] = true;
    }
    s
}

/// A region: its bounding box (px, inclusive) and its pixel count.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    pub area: usize,
}

/// The cell side the regions are computed on, px.
const CELL: usize = 4;

/// The regions of the pixels in `members`, merged within `merge` px, of at
/// least `min_area` pixels. Pixels with rows in `skip` (y0..y1) are ignored.
pub fn regions(
    class: &[u8],
    w: usize,
    h: usize,
    members: &[bool; 256],
    merge: usize,
    min_area: usize,
    skip: Option<(usize, usize)>,
) -> Vec<Region> {
    let (gw, gh) = (w.div_ceil(CELL), h.div_ceil(CELL));
    let mut count = vec![0u32; gw * gh];
    let mut bbox: Vec<[usize; 4]> = vec![[usize::MAX, usize::MAX, 0, 0]; gw * gh];
    for y in 0..h {
        if let Some((a, b)) = skip {
            if y >= a && y < b {
                continue;
            }
        }
        let row = &class[y * w..(y + 1) * w];
        for (x, &c) in row.iter().enumerate() {
            if members[c as usize] {
                let k = (y / CELL) * gw + x / CELL;
                count[k] += 1;
                let b = &mut bbox[k];
                b[0] = b[0].min(x);
                b[1] = b[1].min(y);
                b[2] = b[2].max(x);
                b[3] = b[3].max(y);
            }
        }
    }
    let r = merge.div_ceil(CELL) as isize;
    let mut seen = vec![false; gw * gh];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..gw * gh {
        if count[start] == 0 || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);
        let mut reg = Region {
            x0: usize::MAX,
            y0: usize::MAX,
            x1: 0,
            y1: 0,
            area: 0,
        };
        while let Some(k) = stack.pop() {
            reg.area += count[k] as usize;
            let b = bbox[k];
            reg.x0 = reg.x0.min(b[0]);
            reg.y0 = reg.y0.min(b[1]);
            reg.x1 = reg.x1.max(b[2]);
            reg.y1 = reg.y1.max(b[3]);
            let (cx, cy) = ((k % gw) as isize, (k / gw) as isize);
            for dy in -r..=r {
                for dx in -r..=r {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if nx < 0 || ny < 0 || nx >= gw as isize || ny >= gh as isize {
                        continue;
                    }
                    let n = ny as usize * gw + nx as usize;
                    if count[n] > 0 && !seen[n] {
                        seen[n] = true;
                        stack.push(n);
                    }
                }
            }
        }
        if reg.area >= min_area {
            out.push(reg);
        }
    }
    out
}

/// A rectangle, `[x0, y0, x1, y1]` px (half-open).
pub type Rect = [f64; 4];

fn grow(r: &Rect, d: f64) -> Rect {
    [r[0] - d, r[1] - d, r[2] + d, r[3] + d]
}

fn inside(r: &Rect, x: usize, y: usize) -> bool {
    let (x, y) = (x as f64, y as f64);
    x >= r[0] && x < r[2] && y >= r[1] && y < r[3]
}

/// Ink pixels outside the safe area (or inside a `free` rectangle), not in
/// an `except` rectangle: their count and bounding box.
pub fn outside_safe(
    class: &[u8],
    w: usize,
    h: usize,
    ink: &[bool; 256],
    p: &Profile,
) -> (usize, Option<Region>) {
    let s = &p.safe;
    let mut n = 0;
    let mut bb = Region {
        x0: usize::MAX,
        y0: usize::MAX,
        x1: 0,
        y1: 0,
        area: 0,
    };
    for y in 0..h {
        let yf = y as f64;
        let row_out = yf < s.y[0] - s.bleed || yf >= s.y[1] + s.bleed;
        let row_free = s.free.iter().any(|r| yf >= r[1] && yf < r[3]);
        let row_except = s
            .except
            .iter()
            .any(|r| yf >= r[1] - s.bleed && yf < r[3] + s.bleed);
        let edge = ((s.x[0] - s.bleed).max(0.0) as usize).min(w);
        let edge1 = ((s.x[1] + s.bleed).max(0.0) as usize).min(w);
        let row = &class[y * w..(y + 1) * w];
        for (x, &c) in row.iter().enumerate() {
            if !ink[c as usize] {
                continue;
            }
            let out = row_out
                || x < edge
                || x >= edge1
                || (row_free && s.free.iter().any(|r| inside(r, x, y)));
            if out && !(row_except && s.except.iter().any(|r| inside(&grow(r, s.bleed), x, y))) {
                n += 1;
                bb.x0 = bb.x0.min(x);
                bb.y0 = bb.y0.min(y);
                bb.x1 = bb.x1.max(x);
                bb.y1 = bb.y1.max(y);
            }
        }
    }
    bb.area = n;
    (n, (n > 0).then_some(bb))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_glyphs_close_are_one_region_and_far_are_two() {
        let (w, h) = (200, 40);
        let mut class = vec![0u8; w * h];
        let mut dot = |x0: usize| {
            for y in 10..20 {
                for x in x0..x0 + 10 {
                    class[y * w + x] = 1;
                }
            }
        };
        dot(10);
        dot(30);
        dot(150);
        let m = set(&[1]);
        let rs = regions(&class, w, h, &m, 16, 50, None);
        assert_eq!(rs.len(), 2, "{rs:?}");
        assert_eq!(rs[0].area, 200);
        assert_eq!((rs[0].x0, rs[0].x1), (10, 39));
        // too small to count
        assert!(regions(&class, w, h, &m, 16, 300, None).is_empty());
        // rows skipped
        assert!(regions(&class, w, h, &m, 16, 50, Some((0, 40))).is_empty());
    }
}
