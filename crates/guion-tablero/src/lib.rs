//! `guion-tablero` — the accounts dashboard (R-0013).
//!
//! A workspace is a repo whose root declares its accounts in
//! `guion-cuentas.toml` ([`cuentas`]). The dashboard reads every post of every
//! account ([`post`], [`tablero`]), shows them in a terminal UI ([`ui`]) or as
//! JSON, and runs the existing commands on them as subprocesses
//! ([`acciones`], [`trabajos`]): `guion reel`, its check and its voice. It
//! also appends the owner's one-line ideas ([`encargos`]) and makes new posts
//! from an account's template ([`nuevo`]). It reads one repo or several
//! (`--repo`, or the user's list: [`repos`], R-0014). No brand value lives here.

pub mod acciones;
mod cli;
pub mod cuentas;
pub mod encargos;
pub mod fecha;
pub mod nuevo;
pub mod post;
pub mod repos;
pub mod tablero;
pub mod trabajos;
mod tui;
pub mod ui;

pub use cli::{
    opciones, plan_lote, run, run_encargo, run_nuevo, Opciones, USO, USO_ENCARGO, USO_NUEVO,
};
