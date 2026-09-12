//! El catálogo de modelos: de un nombre a un ejercicio del motor.
//!
//! Ésta es la costura entre las dos clases de trabajo. **Agregar un modelo
//! es trabajo de motor** —va en `physics-lab/mecanica`, con sus
//! afirmaciones— y aparece aquí en una línea. **Componer una comparación
//! es trabajo de creador**, y no toca Rust.
//!
//! El examen del diseño (`docs/DISENO.md`) es que comparar dos ejercicios
//! que ya están en el catálogo no requiera compilar nada nuevo.

use mecanica::{gluteo, Lift};

/// Los parámetros que trae un `[[opcion]]` del guion, ya leídos.
pub struct Params<'a> {
    pub modelo: &'a str,
    pub carga_kg: f64,
    pub rango_grados: f64,
    /// Largo de la palanca, metros. Qué segmento es depende del modelo, y
    /// eso es justamente lo que distingue a un modelo de otro.
    pub palanca_m: f64,
}

/// Qué modelos conoce el catálogo, para poder decirlo en un error.
pub const MODELOS: &[&str] = &["bisagra_de_cadera", "puente_de_cadera"];

/// Construye el ejercicio que pide el guion.
///
/// # Errors
/// Si el modelo no está en el catálogo. No se adivina el más parecido: un
/// modelo equivocado da números plausibles y equivocados, que es peor que
/// un error.
pub fn construir(p: &Params) -> Result<Box<dyn Lift>, String> {
    let rango = p.rango_grados.to_radians();
    match p.modelo {
        // La palanca es el TORSO: el momento muere de pie.
        "bisagra_de_cadera" => Ok(Box::new(gluteo::Rumano {
            load_kg: p.carga_kg,
            torso_m: p.palanca_m,
            bottom_rad: rango,
        })),
        // La palanca es el FÉMUR: el momento pica en el bloqueo.
        "puente_de_cadera" => Ok(Box::new(gluteo::HipThrust {
            load_kg: p.carga_kg,
            femur_m: p.palanca_m,
            bottom_rad: rango,
        })),
        otro => Err(format!(
            "modelo desconocido: {otro:?}. El catálogo tiene: {}",
            MODELOS.join(", ")
        )),
    }
}
