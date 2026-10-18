//! `guion-cuenta` — the account contract and the scaffold of a new account
//! (R-0014).
//!
//! An account is what the shared layer cannot know: its identity and voice,
//! its brand tokens, its scene template, its domain reviewer, its decisions,
//! its status and its deliverables. [`contrato`] is the file that says it
//! (`cuenta.toml`); [`perfil`] derives the gate's profile from it; [`nueva`]
//! scaffolds a whole account from the generic templates in
//! `templates/cuenta/` ([`plantillas`]); [`cli`] is `guion cuenta`.

pub mod cli;
pub mod contrato;
pub mod nueva;
pub mod perfil;
pub mod plantillas;
pub mod texto;

pub use cli::{run, USO};
pub use contrato::Cuenta;
pub use nueva::{crear, plan, Creada, Plan, Respuestas};
