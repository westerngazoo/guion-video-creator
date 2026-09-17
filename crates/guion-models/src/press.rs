//! Press de hombro (militar / Smith), directo del motor.
//!
//! Este módulo llamaba a `guion-fisica`, que es el arnés de valores
//! dorados, no el motor. Ahora llama a `mecanica`, que es donde vive el
//! ejercicio. La diferencia importa: por el camino viejo, un modelo
//! ajustado para pasar una prueba acababa dibujándose en pantalla.

use mecanica::maquina_humana::Persona;
use mecanica::press::{Press, Variante};

/// Qué press y con cuánto.
#[derive(Debug, Clone, Copy)]
pub struct PressParams {
    /// Barra libre o Smith.
    pub variante: Variante,
    /// La barra, kilogramos.
    pub carga: f64,
}

impl PressParams {
    /// Lee los parámetros de un guion.
    #[must_use]
    pub fn from_map(params: &std::collections::BTreeMap<String, f64>) -> Option<Self> {
        let variante = match params.get("variante").copied().unwrap_or(0.0) as i32 {
            0 => Variante::Militar,
            1 => Variante::Smith,
            _ => return None,
        };
        Some(PressParams {
            variante,
            carga: params.get("carga").copied().unwrap_or(40.0),
        })
    }
}

/// Dónde está todo y qué cobra, en un instante del recorrido.
#[derive(Debug, Clone, Copy)]
pub struct PressState {
    /// Avance de la repetición.
    pub u: f64,
    /// Torque en el hombro, N·m.
    pub tau_hombro: f64,
    /// Torque en el codo, N·m.
    pub tau_codo: f64,
    /// El hombro.
    pub shoulder: [f64; 3],
    /// El codo.
    pub elbow: [f64; 3],
    /// La mano.
    pub hand: [f64; 3],
}

/// El estado en el avance `u`.
///
/// # Errors
/// Si la mano cae fuera del alcance del brazo.
pub fn state_at(params: &PressParams, u: f64) -> Result<PressState, String> {
    let p = Press::reel39(
        Persona {
            estatura_m: 1.75,
            masa_kg: 80.0,
        },
        params.carga,
    );
    let pose = p.pose(params.variante, u).map_err(|e| format!("{e:?}"))?;
    Ok(PressState {
        u,
        tau_hombro: p.tau_hombro(&pose),
        tau_codo: p.tau_codo(&pose),
        shoulder: pt(pose.hombro),
        elbow: pt(pose.codo),
        hand: pt(pose.mano),
    })
}

fn pt(p: (f64, f64)) -> [f64; 3] {
    [p.0, p.1, 0.0]
}
