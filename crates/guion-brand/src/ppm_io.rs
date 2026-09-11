use std::io::{self, Write};
use std::path::Path;

use crate::postfx::process_frame;

fn parse_p6_header(data: &[u8]) -> Option<(usize, u32, u32)> {
    if !data.starts_with(b"P6") {
        return None;
    }
    let mut line = 0;
    let mut start = 0;
    for (i, b) in data.iter().enumerate() {
        if *b == b'\n' {
            line += 1;
            if line == 1 {
                start = i + 1;
            } else if line == 2 {
                let dims = std::str::from_utf8(&data[start..i]).ok()?;
                let mut parts = dims.split_whitespace();
                let w = parts.next()?.parse().ok()?;
                let h = parts.next()?.parse().ok()?;
                return Some((i + 1, w, h));
            }
        }
    }
    None
}

/// Read a binary P6 PPM into RGB bytes.
pub fn read_ppm(path: &Path) -> io::Result<(u32, u32, Vec<u8>)> {
    let data = std::fs::read(path)?;
    let (header_end, w, h) = parse_p6_header(&data).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "cabecera PPM inválida")
    })?;
    let rgb = data[header_end..].to_vec();
    Ok((w, h, rgb))
}

/// Apply theme post-fx to every `frame_%05d.ppm` in `dir`.
pub fn postfx_dir(dir: &Path, count: usize, grain: f64, halftone_alpha: u8) -> io::Result<()> {
    for i in 0..count {
        let path = dir.join(format!("frame_{:05}.ppm", i));
        let (w, h, mut rgb) = read_ppm(&path)?;
        process_frame(&mut rgb, w, h, grain, halftone_alpha);
        write_ppm(&path, w, h, &rgb)?;
    }
    Ok(())
}

pub fn write_ppm(path: &Path, w: u32, h: u32, rgb: &[u8]) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write!(f, "P6\n{} {}\n255\n", w, h)?;
    f.write_all(rgb)?;
    Ok(())
}
