use std::path::Path;

use motoreel::Rgb;

use crate::palette::{heat_color, rgb, PaletteFile};

/// Loaded theme: palette + heat ramp + post-fx strengths.
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    palette: PaletteFile,
    pub grain: f64,
    pub halftone_alpha: u8,
}

impl Theme {
    pub fn from_palette(name: impl Into<String>, palette: PaletteFile) -> Self {
        Theme {
            name: name.into(),
            palette,
            grain: 0.04,
            halftone_alpha: 26,
        }
    }

    pub fn stroke_color(&self, name: &str, heat_value: Option<f64>) -> Rgb {
        if name.starts_with("heat:") {
            let t = heat_value.unwrap_or(0.5);
            return heat_color(&self.palette.heat, t);
        }
        match name {
            "skin" => rgb(self.palette.colors.skin),
            "ink" => rgb(self.palette.colors.ink),
            "paper" => rgb(self.palette.colors.paper),
            "accent" => rgb(self.palette.colors.accent),
            "accent2" => rgb(self.palette.colors.accent2),
            _ => rgb(self.palette.colors.ink),
        }
    }

    pub fn heat_ref_force(&self) -> f64 {
        1600.0
    }
}

pub fn load_theme(path: &Path) -> Result<Theme, String> {
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let palette = PaletteFile::parse(&src).map_err(|e| e.to_string())?;
    let name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "default".to_string());
    Ok(Theme::from_palette(name, palette))
}

pub fn default_theme() -> Theme {
    Theme::from_palette(
        "fbf",
        PaletteFile::parse(include_str!("../../../themes/fbf/palette.toml")).expect("fbf palette"),
    )
}
