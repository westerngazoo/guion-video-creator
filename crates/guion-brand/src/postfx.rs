//! Frame post-processing: grain + halftone dots (ported from `tools/fabrica.py`).

/// Apply grain and halftone to an RGB raster (row-major, `width * height * 3` bytes).
pub fn process_frame(rgb: &mut [u8], width: u32, height: u32, grain: f64, halftone_alpha: u8) {
    if grain > 0.0 {
        apply_grain(rgb, grain);
    }
    if halftone_alpha > 0 {
        apply_halftone(rgb, width, height, halftone_alpha);
    }
}

fn apply_grain(rgb: &mut [u8], amount: f64) {
    let mut state = 0x9e37_79b9_u32;
    for i in (0..rgb.len()).step_by(3) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        let n = (state % 41) as i16 - 20;
        for c in 0..3 {
            let v = rgb[i + c] as i16 + (n as f64 * amount) as i16;
            rgb[i + c] = v.clamp(0, 255) as u8;
        }
    }
}

fn apply_halftone(rgb: &mut [u8], width: u32, height: u32, alpha: u8) {
    let step = 18u32;
    let r = 2.3;
    let ink = 20u8;
    for y in (0..height).step_by(step as usize) {
        let off = if (y / step) % 2 == 1 { step / 2 } else { 0 };
        for x in (0..width).step_by(step as usize) {
            let cx = x as f64 + off as f64;
            let cy = y as f64;
            stamp_dot(rgb, width, height, cx, cy, r, ink, alpha);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stamp_dot(
    rgb: &mut [u8],
    width: u32,
    height: u32,
    cx: f64,
    cy: f64,
    r: f64,
    ink: u8,
    alpha: u8,
) {
    let r_i = r.ceil() as i32;
    for dy in -r_i..=r_i {
        for dx in -r_i..=r_i {
            if (dx * dx + dy * dy) as f64 > r * r {
                continue;
            }
            let x = cx as i32 + dx;
            let y = cy as i32 + dy;
            if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                continue;
            }
            let i = (y as u32 * width + x as u32) as usize * 3;
            for c in 0..3 {
                let base = rgb[i + c];
                let t = alpha as f64 / 255.0;
                rgb[i + c] = ((1.0 - t) * base as f64 + t * ink as f64).round() as u8;
            }
        }
    }
}
