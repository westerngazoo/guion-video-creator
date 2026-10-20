//! `guion-reel`: the studio for reels written as code (R-0009).
//!
//! A screenplay (guion-core) declares a scene; some scenes do not fit a
//! declarative schema (a low-poly figure computed with geometric algebra, a
//! page that translates itself sentence by sentence). Those are written as a
//! pure function of time that returns motoreel primitives. This crate is
//! everything around that function, shared by every studio that works that
//! way:
//!
//! - [`Canvas`]: the pixel size, the frame rate and the map to view units;
//! - [`Clock`]: the named instants and sampled curves, written as
//!   `events.json` so the sound design reads the same numbers the picture
//!   used (AC1);
//! - [`Reel`]: the MP4 in parallel and byte-identical to one thread (AC2),
//!   review frames and the contact sheet (AC3), raster overlays (AC4), and
//!   [`Command`] to pick among them from a command line;
//! - [`Narration`]: the voice's timeline (`voz.json`), the clock of a
//!   narrated reel (AC5);
//! - [`subtitle`]: subtitle layout, measured by the caller (AC6);
//! - [`gate`]: the automatic gate of a narrated reel, against a brand
//!   profile the brand writes (R-0012);
//! - [`Formato`]: the delivery formats (reel, TikTok, YouTube Short, long
//!   YouTube video), their canvases and limits, and [`youtube`]: the `.srt`
//!   and the thumbnail (R-0016).
//!
//! Nothing here belongs to a brand (AC7): faces, colours, sizes and the
//! folder layout are the caller's.

pub mod canvas;
pub mod clock;
pub mod ease;
pub mod error;
pub mod formato;
pub mod gate;
mod json;
pub mod narration;
pub mod studio;
pub mod subtitle;
pub mod youtube;

pub use canvas::Canvas;
pub use clock::{Clock, Series};
pub use error::ReelError;
pub use formato::Formato;
pub use narration::{NarratedLine, Narration, Sentence, COMMA_PAUSE};
pub use studio::{
    all_cores, png_from_rgb, times, Command, FontLoader, Frame, Layout, NoOverlay, Overlay, Reel,
    SheetGrid,
};
