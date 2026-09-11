use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Dorados {
    pub piezas: BTreeMap<String, Pieza>,
}

#[derive(Debug, Deserialize)]
pub struct Pieza {
    pub constantes: BTreeMap<String, serde_json::Value>,
    pub muestras: Vec<Muestra>,
    #[serde(default)]
    pub descripcion: String,
}

#[derive(Debug, Deserialize)]
pub struct Muestra {
    pub entrada: BTreeMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub salida: BTreeMap<String, serde_json::Value>,
}

/// Path to `guion/fixtures/dorados.json` from this crate.
pub fn default_dorados_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/dorados.json")
}

pub fn load(path: &Path) -> Result<Dorados, String> {
    let bytes = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&bytes).map_err(|e| e.to_string())
}

/// Relative error against golden, within `tol` fraction (R-0001 Q5: 2 %).
pub fn within_tol(got: f64, expected: f64, tol: f64) -> bool {
    if !got.is_finite() || !expected.is_finite() {
        return false;
    }
    let denom = expected.abs().max(1e-9);
    (got - expected).abs() / denom <= tol
}
