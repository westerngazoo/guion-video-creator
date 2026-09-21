//! `guion-brand` — themes, palettes, and post-fx (M3).

pub mod fuentes;
mod palette;
mod postfx;
mod ppm_io;
mod theme;

pub use palette::{heat_color, PaletteFile};
pub use postfx::process_frame;
pub use ppm_io::postfx_dir;
pub use theme::{default_theme, load_theme, Theme};
