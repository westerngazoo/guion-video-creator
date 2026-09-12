//! El guion: lo que escribe el creador. TOML, sin código.

use serde::Deserialize;

use crate::catalogo::{construir, Params};
use crate::{Comparacion, Criterio, Medida, Opcion, Sentido};

#[derive(Debug, Deserialize)]
struct Archivo {
    comparacion: Cab,
    opcion: Vec<Op>,
    #[serde(default)]
    criterio: Vec<Crit>,
}

#[derive(Debug, Deserialize)]
struct Cab {
    titulo: String,
    objetivo: String,
    carga_kg: f64,
}

#[derive(Debug, Deserialize)]
struct Op {
    nombre: String,
    modelo: String,
    palanca_m: f64,
    rango_grados: f64,
}

#[derive(Debug, Deserialize)]
struct Crit {
    etiqueta: String,
    medida: String,
    #[serde(default)]
    en: Option<f64>,
    sentido: String,
    #[serde(default = "empate_por_omision")]
    empate_si: f64,
}

/// 15%: por debajo de eso, dos opciones de gimnasio no se distinguen en la
/// práctica y coronar a una sería presentar ruido como hallazgo.
fn empate_por_omision() -> f64 {
    0.15
}

/// Lee un guion y arma la comparación.
///
/// # Errors
/// TOML inválido, modelo fuera del catálogo, o menos de dos opciones —
/// una comparación de una sola cosa no es una comparación.
pub fn cargar(texto: &str) -> Result<Comparacion, String> {
    let a: Archivo = toml::from_str(texto).map_err(|e| e.to_string())?;
    if a.opcion.len() < 2 {
        return Err(format!(
            "una comparación necesita al menos dos opciones, trae {}",
            a.opcion.len()
        ));
    }
    let mut opciones = Vec::with_capacity(a.opcion.len());
    for o in &a.opcion {
        opciones.push(Opcion {
            nombre: o.nombre.clone(),
            lift: construir(&Params {
                modelo: &o.modelo,
                carga_kg: a.comparacion.carga_kg,
                rango_grados: o.rango_grados,
                palanca_m: o.palanca_m,
            })?,
        });
    }
    let mut criterios = Vec::with_capacity(a.criterio.len());
    for c in &a.criterio {
        let medida = match c.medida.as_str() {
            "pico" => Medida::Pico,
            "trabajo" => Medida::Trabajo,
            "en_progreso" => Medida::EnProgreso(c.en.ok_or_else(|| {
                format!("el criterio {:?} es 'en_progreso' y le falta `en`", c.etiqueta)
            })?),
            otro => return Err(format!("medida desconocida: {otro:?}")),
        };
        let sentido = match c.sentido.as_str() {
            "mayor_es_mejor" => Sentido::MayorEsMejor,
            "menor_es_mejor" => Sentido::MenorEsMejor,
            "informativo" => Sentido::Informativo,
            otro => return Err(format!("sentido desconocido: {otro:?}")),
        };
        criterios.push(Criterio {
            etiqueta: c.etiqueta.clone(),
            medida,
            sentido,
            empate_si: c.empate_si,
        });
    }
    Ok(Comparacion {
        titulo: a.comparacion.titulo.clone(),
        objetivo: a.comparacion.objetivo.clone(),
        opciones,
        criterios,
    })
}
