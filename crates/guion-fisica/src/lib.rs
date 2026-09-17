//! `guion-fisica` — physics ported from published reels, verified against
//! `fixtures/dorados.json` (ENCARGO §3, tolerancia 2 %).

mod dorados;
mod reel04;
mod reel26;
mod reel29;
mod reel39;

pub use dorados::{
    default_dorados_path, load as load_dorados, within_tol, Dorados, Muestra, Pieza,
};
pub use reel04::{Modo, Reel04};
pub use reel26::Reel26;
pub use reel29::Reel29;
pub use reel39::{Pose2, Reel39, Variante as Reel39Variante};

/// Default relative tolerance for golden comparisons (R-0001 Q5).
pub const DORADOS_TOL: f64 = 0.02;

/// Evaluate one `reel04` golden sample (`tau` in N·m).
pub fn eval_reel04(muestra: &Muestra) -> Result<f64, String> {
    let phi_grados = json_f64(muestra.entrada.get("phi_grados"), "phi_grados")?;
    let modo = json_str(muestra.entrada.get("modo"), "modo")?;
    let modo = Modo::parse(&modo).ok_or_else(|| format!("modo desconocido: {modo}"))?;
    let phi = phi_grados.to_radians();
    Ok(Reel04::default().tau(phi, modo))
}

/// Evaluate one `reel26` golden sample (`tau` in N·m at stroke end).
pub fn eval_reel26(muestra: &Muestra) -> Result<f64, String> {
    let beta_grados = json_f64(muestra.entrada.get("beta_grados"), "beta_grados")?;
    Ok(Reel26::default().tau(beta_grados.to_radians()))
}

/// Evaluate one `reel29` golden sample (`tau`, `largo_cable`, or `recorrido`).
pub fn eval_reel29(muestra: &Muestra) -> Result<f64, String> {
    let reel = Reel29::default();
    let beta_grados = json_f64(muestra.entrada.get("beta_grados"), "beta_grados")?;
    let beta = beta_grados.to_radians();

    if let Some(medida) = muestra.entrada.get("medida") {
        let medida = json_str(Some(medida), "medida")?;
        if medida == "recorrido" {
            return Ok(reel.recorrido(beta));
        }
        return Err(format!("medida desconocida: {medida}"));
    }

    let u = json_f64(muestra.entrada.get("u"), "u")?;

    if muestra.salida.contains_key("largo_cable") {
        return Ok(reel.largo_cable(beta, u));
    }
    if muestra.salida.contains_key("tau") {
        return Ok(reel.tau(beta, u));
    }
    Err("salida reel29: se esperaba tau o largo_cable".into())
}

/// Evaluate one output field of a `reel39` golden sample.
pub fn eval_reel39_field(muestra: &Muestra, field: &str) -> Result<f64, String> {
    let reel = Reel39::default();
    let variante = json_str(muestra.entrada.get("variante"), "variante")?;
    let variante = Reel39Variante::parse(&variante)
        .ok_or_else(|| format!("variante desconocida: {variante}"))?;

    if let Some(medida) = muestra.entrada.get("medida") {
        let medida = json_str(Some(medida), "medida")?;
        if medida == "balance de energía" {
            return match field {
                "trabajo_musculos" | "trabajo_carga" => Ok(reel.trabajo_carga()),
                other => Err(format!("campo {other} no aplica a balance de energía")),
            };
        }
        return Err(format!("medida reel39 no implementada: {medida}"));
    }

    if muestra.entrada.get("x").is_some() {
        let x = json_f64(muestra.entrada.get("x"), "x")?;
        return match field {
            "tau_hombro_medio" | "pico_hombro" => Ok(reel.tau_hombro_offset(x)),
            other => Err(format!("campo {other} no aplica a sensibilidad x")),
        };
    }

    if muestra.entrada.contains_key("x0") {
        let x0 = json_f64(muestra.entrada.get("x0"), "x0")?;
        return match field {
            "pico_hombro" => Ok(reel.tau_hombro_offset(x0)),
            "tau_hombro_final" => Ok(0.0),
            other => Err(format!("campo {other} no aplica a sensibilidad x0")),
        };
    }

    let u = json_f64(muestra.entrada.get("u"), "u")?;
    let pose = reel.pose(variante, u);

    match field {
        "hombro" => Ok(reel.tau_hombro(&pose)),
        "codo" => Ok(reel.tau_codo(&pose)),
        "hombro_grados" => Ok(reel.hombro_grados(&pose)),
        "codo_grados" => Ok(reel.codo_grados(&pose)),
        other => Err(format!("campo reel39 desconocido: {other}")),
    }
}

fn json_f64(v: Option<&serde_json::Value>, key: &str) -> Result<f64, String> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or_else(|| format!("{key}: no es f64")),
        _ => Err(format!("{key}: falta o tipo inválido")),
    }
}

fn json_str(v: Option<&serde_json::Value>, key: &str) -> Result<String, String> {
    match v {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{key}: falta o no es string")),
    }
}
