//! `guion-motion` — resolve `physics-lab` wasm models (M4 `InPlace` source).

mod error;
mod lever;
mod wasm;

pub use error::MotionError;
pub use wasm::{physics_lab_root, wasm_path_for_source, WasmLesson};

use guion_core::{ModelRef, Screenplay, Token};
use guion_models::{
    resolve_point as native_point, resolve_token as native_token, LeverParams, LeverState,
};

/// Cached wasm lesson per model id.
pub struct ModelRuntime {
    root: std::path::PathBuf,
    lever: Option<WasmLesson>,
}

impl Default for ModelRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRuntime {
    pub fn new() -> Self {
        ModelRuntime {
            root: wasm::physics_lab_root(),
            lever: None,
        }
    }

    pub fn with_root(root: std::path::PathBuf) -> Self {
        ModelRuntime { root, lever: None }
    }

    fn lever_lesson(&mut self) -> Result<&mut WasmLesson, MotionError> {
        if self.lever.is_none() {
            let path =
                wasm::wasm_path_for_source("physics-lab:lever", &self.root).ok_or_else(|| {
                    MotionError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "lesson.wasm no encontrado para lever",
                    ))
                })?;
            self.lever = Some(WasmLesson::load(&path, 5)?);
        }
        Ok(self.lever.as_mut().unwrap())
    }

    pub fn resolve_token(&mut self, sp: &Screenplay, token: &Token, phi: f64) -> Option<f64> {
        let model = sp.model.iter().find(|m| m.id == token.id())?;
        if model.source == "physics-lab:lever" {
            if let Ok(lesson) = self.lever_lesson() {
                if let Ok(read) = lever::eval_wasm(lesson, model, phi) {
                    return lever::scalar_from_readouts(token.field(), &read, phi);
                }
            }
        }
        if es_press(&model.source) {
            return native_token(sp, token, phi);
        }
        native_token(sp, token, phi)
    }

    pub fn resolve_point(&mut self, sp: &Screenplay, token: &Token, phi: f64) -> Option<[f64; 3]> {
        native_point(sp, token, phi)
    }

    pub fn lever_state(&self, model: &ModelRef, phi: f64) -> Option<LeverState> {
        let p = LeverParams::from_map(&model.params)?;
        Some(p.state_at(phi))
    }
}

/// Stateless convenience — tries wasm, falls back to native.
pub fn resolve_token(sp: &Screenplay, token: &Token, phi: f64) -> Option<f64> {
    let mut rt = ModelRuntime::new();
    rt.resolve_token(sp, token, phi)
}

pub fn resolve_point(sp: &Screenplay, token: &Token, phi: f64) -> Option<[f64; 3]> {
    let mut rt = ModelRuntime::new();
    rt.resolve_point(sp, token, phi)
}

// El nombre nuevo dice de dónde sale la física de verdad. El viejo se
// sigue aceptando: hay guiones escritos con él, y romperlos por un
// renombre sería cobrarle al autor un problema nuestro.
fn es_press(fuente: &str) -> bool {
    fuente == "mecanica:press" || fuente == "guion-fisica:reel39"
}
