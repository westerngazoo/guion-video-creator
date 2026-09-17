//! reel39 — press de hombro (militar / Smith).

use guion_fisica::{Reel39, Reel39Variante};

#[derive(Debug, Clone, Copy)]
pub struct PressParams {
    pub variante: Reel39Variante,
    pub carga: f64,
}

impl PressParams {
    pub fn from_map(params: &std::collections::BTreeMap<String, f64>) -> Option<Self> {
        let variante = match params.get("variante").copied().unwrap_or(0.0) as i32 {
            0 => Reel39Variante::Militar,
            1 => Reel39Variante::Smith,
            _ => return None,
        };
        Some(PressParams {
            variante,
            carga: params.get("carga").copied().unwrap_or(40.0),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PressState {
    pub u: f64,
    pub tau_hombro: f64,
    pub tau_codo: f64,
    pub shoulder: [f64; 3],
    pub elbow: [f64; 3],
    pub hand: [f64; 3],
}

pub fn state_at(params: &PressParams, u: f64) -> PressState {
    let reel = Reel39 {
        carga: params.carga,
        ..Reel39::default()
    };
    let pose = reel.pose(params.variante, u);
    PressState {
        u,
        tau_hombro: reel.tau_hombro(&pose),
        tau_codo: reel.tau_codo(&pose),
        shoulder: pt(pose.hombro),
        elbow: pt(pose.codo),
        hand: pt(pose.mano),
    }
}

fn pt(p: (f64, f64)) -> [f64; 3] {
    [p.0, p.1, 0.0]
}
