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

    /// El suelo de la página: el color con el que se limpia cada cuadro
    /// antes de dibujar nada.
    ///
    /// R-0008 OQ-1. Existe como accesor propio y no como
    /// `stroke_color("paper")` porque no es un trazo: es lo único que se
    /// ve donde no hay trazo. Pedirlo por la puerta de los trazos fue
    /// justo lo que dejó que nadie lo pidiera nunca.
    pub fn paper(&self) -> Rgb {
        rgb(self.palette.colors.paper)
    }

    /// El suelo del cintillo del pie, la segunda superficie de la pieza.
    pub fn bar(&self) -> Rgb {
        rgb(self.palette.colors.bar)
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

/// La marca por omisión. Pasa por `theme_by_name` para que exista UNA
/// tabla de marcas y no dos que se puedan desincronizar.
pub fn default_theme() -> Theme {
    theme_by_name(MARCA_POR_OMISION).expect("la marca por omisión")
}

/// Las marcas que este binario trae adentro, en el orden en que se
/// ofrecen cuando alguien escribe un nombre que no existe.
///
/// Van embebidas y no leídas de disco por la misma razón que `fbf` ya lo
/// estaba: el render tiene que dar el mismo resultado desde cualquier
/// directorio, y una marca que depende del cwd es una marca que algún día
/// no se encuentra y se cae al tema por omisión sin avisar. Para temas de
/// fuera del repo está `load_theme`, que sí toma una ruta.
const MARCAS: &[(&str, &str)] = &[
    ("fbf", include_str!("../../../themes/fbf/palette.toml")),
    ("pista", include_str!("../../../themes/pista/palette.toml")),
];

/// Resuelve una marca por nombre.
///
/// R-0008 AC2: un nombre que no existe es un ERROR que nombra las marcas
/// que sí, nunca un regreso callado a la marca por omisión. El defecto que
/// esto cierra no es hipotético: el mismo silencio, en la fábrica vieja,
/// dejó una pieza renderizada entera con la marca equivocada y ninguna de
/// las 41 comprobaciones lo dijo, porque ninguna preguntaba por la marca.
/// Un render con la identidad de otro cliente se ve terminado; ése es
/// justo el fallo que tiene que gritar.
pub fn theme_by_name(name: &str) -> Result<Theme, String> {
    for (n, src) in MARCAS {
        if *n == name {
            let palette =
                PaletteFile::parse(src).map_err(|e| format!("la marca `{name}` no carga: {e}"))?;
            return Ok(Theme::from_palette(*n, palette));
        }
    }
    let hay: Vec<&str> = MARCAS.iter().map(|(n, _)| *n).collect();
    Err(format!(
        "no existe la marca `{name}`. Las que hay: {}",
        hay.join(", ")
    ))
}

/// El nombre de la marca por omisión cuando el guion no dice ninguna.
pub const MARCA_POR_OMISION: &str = "fbf";
