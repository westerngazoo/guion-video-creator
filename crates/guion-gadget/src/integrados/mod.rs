//! The built-in gadgets (R-0015 AC2): the first two, ported from real reels
//! to prove the contract. Phase 2 adds the rest of the prototype's library
//! (docs/specs/0015-studio.md §8).

pub mod medidor;
pub mod palanca;

pub use medidor::Medidor;
pub use palanca::Palanca;
