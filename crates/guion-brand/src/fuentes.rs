//! Las caras con las que se escribe. R-0009, del lado del productor.
//!
//! `motoreel` ya no trae una tipografía adentro: dejó de tener un mapa de
//! 5 × 7 sobre ASCII —que convertía cada acento en `'?'` y por eso el
//! motor escribía «b?ceps»— y ahora pide un registro de caras reales. Ese
//! registro lo arma quien produce, que es este repo, porque la marca es
//! suya y no del motor.
//!
//! **Y si no hay cara, no hay cuadro.** Un `PpmSink` sin registro rechaza
//! un primitivo de texto en vez de dibujar nada. Es a propósito: dibujar
//! nada es el mismo defecto con otro disfraz.

use std::path::{Path, PathBuf};

use motoreel_typeset::{Face, Fonts};

/// Los archivos de la marca, en el orden en que `face: usize` los indexa.
///
/// El índice **importa**: un guion que pide la cara 1 pide la segunda de
/// esta lista. Reordenarla cambia lo que sale en pantalla sin cambiar una
/// sola línea de guion, así que se agrega al final.
const CARAS: &[(&str, &str)] = &[
    ("mono", "ComicNeue-Bold.ttf"),
    ("display", "Bangers.ttf"),
    ("display-alt", "LuckiestGuy.ttf"),
];

/// Por qué no se pudo armar el registro.
#[derive(Debug)]
pub enum Error {
    /// No se encontró el directorio de fuentes en ningún lado.
    SinDirectorio(Vec<PathBuf>),
    /// Falta un archivo dentro del directorio que sí existe.
    Falta {
        /// El archivo que no está.
        archivo: PathBuf,
        /// El rol que iba a cumplir.
        rol: &'static str,
    },
    /// El archivo existe pero no es una fuente que se pueda leer.
    NoEsFuente(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SinDirectorio(probados) => write!(
                f,
                "no encontré las fuentes de la marca. Probé {probados:?}. \
                 Pon GUION_FONTS apuntando al directorio que las tiene"
            ),
            Error::Falta { archivo, rol } => {
                write!(f, "falta {} — es la cara «{rol}»", archivo.display())
            }
            Error::NoEsFuente(p) => write!(f, "{} no es una fuente legible", p.display()),
        }
    }
}

impl std::error::Error for Error {}

/// Dónde buscar, en orden.
///
/// Las fuentes **todavía no viven en este repo**: son de
/// `fisicobuenfisico/brand/fonts/`, y ahí están sin su archivo de
/// licencia. Copiarlas aquí es lo correcto y es trabajo aparte — la OFL
/// pide que la licencia viaje con la fuente, y no se copia un archivo
/// licenciado sin ella. Mientras tanto se leen de donde están.
fn candidatos() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = std::env::var_os("GUION_FONTS") {
        v.push(PathBuf::from(dir));
    }
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(raiz) = raiz {
        v.push(raiz.join("themes/fbf/fonts"));
        if let Some(arriba) = raiz.parent() {
            v.push(arriba.join("fisicobuenfisico/brand/fonts"));
        }
    }
    v
}

/// El registro de la marca, listo para `PpmSink::with_fonts`.
///
/// # Errors
/// [`Error`] cuando no hay directorio, falta un archivo o uno no se puede
/// leer como fuente. Nada de esto se resuelve con una cara de repuesto:
/// una cara de repuesto es un `'?'` con sombrero — el cuadro sale, con los
/// glifos equivocados.
pub fn marca() -> Result<Fonts, Error> {
    let probados = candidatos();
    let dir = probados
        .iter()
        .find(|d| d.join(CARAS[0].1).exists())
        .ok_or_else(|| Error::SinDirectorio(probados.clone()))?;

    let mut fonts = Fonts::new();
    for (rol, archivo) in CARAS {
        let ruta = dir.join(archivo);
        let bytes = std::fs::read(&ruta).map_err(|_| Error::Falta {
            archivo: ruta.clone(),
            rol,
        })?;
        let face = Face::load(bytes, (*rol).to_string()).map_err(|_| Error::NoEsFuente(ruta))?;
        fonts.add(face);
    }
    Ok(fonts)
}

/// Los nombres de familia para el sink de SVG, en el mismo orden.
///
/// SVG no carga el archivo: nombra una familia y el motor de fuentes del
/// consumidor la resuelve. Por eso aquí va el nombre y no la ruta.
#[must_use]
pub fn familias() -> Vec<&'static str> {
    CARAS.iter().map(|(rol, _)| *rol).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_indice_de_cada_cara_es_estable() {
        // Un guion que pide `face: 1` pide «display». Si alguien reordena
        // `CARAS`, el video cambia sin que cambie el guion — esta prueba
        // es la que lo cuenta.
        assert_eq!(familias(), ["mono", "display", "display-alt"]);
    }

    #[test]
    fn sin_fuentes_el_error_dice_dónde_buscó() {
        let e = Error::SinDirectorio(vec![PathBuf::from("/no/existe")]);
        let s = e.to_string();
        assert!(s.contains("/no/existe"), "{s}");
        assert!(s.contains("GUION_FONTS"), "{s}");
    }

    #[test]
    fn la_marca_carga_y_escribe_español() {
        let Ok(fonts) = marca() else {
            // En una máquina sin el repo de contenido al lado no hay nada
            // que probar; el error ya se prueba arriba.
            return;
        };
        assert_eq!(fonts.len(), CARAS.len());
        for i in 0..fonts.len() {
            let face = fonts.get(motoreel_typeset::FaceId(i)).expect("registrada");
            assert!(
                face.missing("bíceps ángulo tensión 30° ¿QUÉ?").is_empty(),
                "la cara {i} no puede escribir español"
            );
        }
    }
}
