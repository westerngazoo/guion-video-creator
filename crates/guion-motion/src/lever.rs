use guion_core::ModelRef;
use guion_models::LeverParams;

use crate::wasm::WasmLesson;

/// Param order matches `public/lessons/lever/lesson.json`.
pub fn params_from_model(model: &ModelRef, phi: f64) -> Option<Vec<f64>> {
    let p = LeverParams::from_map(&model.params)?;
    Some(vec![p.load_mass, p.forearm, p.insertion, p.upper_arm, phi])
}

/// Readout slots from lever lesson.json.
pub fn scalar_from_readouts(field: Option<&str>, read: &[f64; 8], phi: f64) -> Option<f64> {
    match field {
        None | Some("phi") => Some(phi),
        Some("biceps_force_N") => Some(read[0]),
        Some("load_force_N") => Some(read[1]),
        Some("torque_Nm") => Some(read[2]),
        _ => None,
    }
}

pub fn eval_wasm(
    lesson: &mut WasmLesson,
    model: &ModelRef,
    phi: f64,
) -> Result<[f64; 8], crate::error::MotionError> {
    let params = params_from_model(model, phi)
        .ok_or_else(|| crate::error::MotionError::UnknownField("lever params".into()))?;
    lesson.eval(&params)
}
