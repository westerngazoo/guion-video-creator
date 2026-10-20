//! `guion-gadget`: the studio's building blocks (R-0015).
//!
//! A reel is assembled from **gadgets**: a lever with its torque, a gauge, a
//! live graph, a 3D mesh under a camera. Each is a pure function of time and
//! typed params, so the studio can show it live, the render can draw it
//! frame by frame, and the same frame always has the same bytes.
//!
//! - [`valor`]: typed params ([`ParamSpec`]: range, unit) and values with
//!   their provenance ([`Procedencia`]: calculado, medido, reportado,
//!   supuesto) (AC1);
//! - [`gadget`]: the [`Gadget`] contract, its [`Contexto`] (canvas, palette
//!   by role, faces) and what `eval` returns ([`Salida`]: prims, eventos,
//!   valores, avisos) (AC1);
//! - [`registro`]: the [`Registro`] of built-in and account gadgets and the
//!   contract test every one passes ([`registro::comprobar_contrato`]) (AC3);
//! - [`linea`]: anchors to the voice and animations (AC4);
//! - [`escena`]: `escena.toml`, its validation, a frame at `t`, the clock
//!   and the canonical text the studio saves (AC5);
//! - [`dibujo`]: drawing in canvas pixels and label placement by obstacle
//!   avoidance;
//! - [`integrados`]: the built-in gadgets, «palanca» and «medidor» (AC2).
//!
//! The physics is physics-lab's (`mecanica`): a gadget calls it and never
//! re-derives it.

pub mod dibujo;
pub mod escena;
pub mod gadget;
pub mod integrados;
pub mod linea;
pub mod registro;
pub mod valor;

pub use escena::{Compilada, Cuadro, Entrada, Escena, Estado, Origen};
pub use gadget::{Categoria, Contexto, Gadget, Oraculo, Paleta, Salida};
pub use linea::{Curva, Tiempos};
pub use registro::{comprobar_contrato, Registro};
pub use valor::{Dato, ParamSpec, Params, Procedencia, SalidaSpec, Tipo, Valor};
