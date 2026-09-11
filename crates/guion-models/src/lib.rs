//! `guion-models` — native physics for M1 assembly.
//!
//! M4 will load compiled `physics-lab` wasm; until then this crate supplies
//! the closed-form models referenced by golden screenplays.

mod lever;
mod press;

pub use lever::{LeverParams, LeverState};
pub use press::{PressParams, PressState};

use guion_core::{ModelRef, Screenplay, Token};

/// Resolve a model output token against the screenplay at time `phi` (radians).
pub fn resolve_token(sp: &Screenplay, token: &Token, phi: f64) -> Option<f64> {
    let model = sp.model.iter().find(|m| m.id == token.id())?;
    resolve_model_field(model, token.field(), phi)
}

/// Resolve a model output token to a 3D point.
pub fn resolve_point(sp: &Screenplay, token: &Token, phi: f64) -> Option<[f64; 3]> {
    let model = sp.model.iter().find(|m| m.id == token.id())?;
    if let Some(state) = press_state(model, phi) {
        return point_field_press(&state, token.field());
    }
    let state = lever_state(model, phi)?;
    point_field(&state, token.field())
}

fn lever_state(model: &ModelRef, phi: f64) -> Option<LeverState> {
    if model.source != "physics-lab:lever" {
        return None;
    }
    let params = LeverParams::from_map(&model.params)?;
    Some(params.state_at(phi))
}

fn press_state(model: &ModelRef, u: f64) -> Option<PressState> {
    if model.source != "guion-fisica:reel39" {
        return None;
    }
    let params = PressParams::from_map(&model.params)?;
    Some(press::state_at(&params, u))
}

fn resolve_model_field(model: &ModelRef, field: Option<&str>, phi: f64) -> Option<f64> {
    if let Some(state) = press_state(model, phi) {
        return match field {
            None | Some("u") => Some(state.u),
            Some("tau_hombro") => Some(state.tau_hombro),
            Some("tau_codo") => Some(state.tau_codo),
            _ => None,
        };
    }
    let state = lever_state(model, phi)?;
    match field {
        None | Some("phi") => Some(state.phi),
        Some("biceps_force_N") => Some(state.biceps_force_n),
        Some("load_force_N") => Some(state.load_force_n),
        Some("torque_Nm") => Some(state.torque_nm),
        _ => None,
    }
}

fn point_field_press(state: &PressState, field: Option<&str>) -> Option<[f64; 3]> {
    match field {
        Some("shoulder") => Some(state.shoulder),
        Some("elbow") => Some(state.elbow),
        Some("hand") => Some(state.hand),
        _ => None,
    }
}

fn point_field(state: &LeverState, field: Option<&str>) -> Option<[f64; 3]> {
    match field {
        Some("elbow") => Some(state.elbow),
        Some("hand") => Some(state.hand),
        Some("insertion") => Some(state.insertion),
        Some("shoulder") => Some(state.shoulder),
        _ => None,
    }
}
