//! `guion-marca`: a brand as data (R-0014, phase 1).
//!
//! An account declares its tokens in `cuenta.toml` (guion-cuenta's contract);
//! [`Marca`] reads them and gives a scene the neutral primitives every reel
//! needs: the background, text in the account's faces sized in em as the gate
//! measures it, the title, the subtitle of the line being said, an accent bar,
//! the closing card, the clock of the voice and the studio ([`Marca::reel`]).
//! [`sonido`] adds neutral instruments for the
//! score (R-0011), so `guion reel sonido` makes a new account's bed.
//!
//! The crate re-exports guion-reel ([`reel`]) and motoreel, so a scene
//! depends on this crate alone. Nothing here is a brand: change the tokens
//! and the same scene is another account.

pub mod marca;
pub mod sonido;

pub use guion_cuenta::Cuenta;
pub use guion_reel as reel;
pub use marca::{con_alfa, Cara, Marca};
pub use motoreel;
pub use motoreel::{Align, Prim2, Rgb};
