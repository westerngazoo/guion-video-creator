//! The canvas: pixel size, frame rate, and the map from canvas pixels to
//! motoreel's view units.
//!
//! Scenes are laid out in canvas pixels with y pointing down, the way a design
//! is drawn; motoreel's view has its origin at the centre and y pointing up.
//! [`Canvas::to_view`] is the one place that conversion lives.

use guion_encode::VideoSpec;
use motoreel::Pt2;

/// A canvas: `width` × `height` px at `fps`, with `px_per_unit` canvas pixels
/// per motoreel view unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub px_per_unit: f64,
}

impl Canvas {
    /// A vertical reel: 1080 × 1920 at 30 fps, 600 px per unit (a 1.8 × 3.2 view).
    pub const VERTICAL_REEL: Canvas = Canvas {
        width: 1080,
        height: 1920,
        fps: 30.0,
        px_per_unit: 600.0,
    };

    /// A long YouTube video (R-0016): 1920 × 1080 at 30 fps, 600 px per unit
    /// (a 3.2 × 1.8 view: the vertical reel turned on its side).
    pub const HORIZONTAL_VIDEO: Canvas = Canvas {
        width: 1920,
        height: 1080,
        fps: 30.0,
        px_per_unit: 600.0,
    };

    /// The view window the sink rasterizes, in units.
    pub fn view(&self) -> (f64, f64) {
        (
            self.width as f64 / self.px_per_unit,
            self.height as f64 / self.px_per_unit,
        )
    }

    /// A canvas point (px, y down) in view units (origin at the centre, y up).
    pub fn to_view(&self, x: f64, y: f64) -> Pt2 {
        Pt2 {
            x: (x - self.width as f64 / 2.0) / self.px_per_unit,
            y: (self.height as f64 / 2.0 - y) / self.px_per_unit,
        }
    }

    /// A length in canvas pixels, in view units.
    pub fn len(&self, px: f64) -> f64 {
        px / self.px_per_unit
    }

    /// The number of frames in `duration` seconds (rounded to the nearest frame).
    pub fn frames(&self, duration: f64) -> usize {
        (duration * self.fps).round() as usize
    }

    /// What the encoder needs to know.
    pub fn video_spec(&self) -> VideoSpec {
        VideoSpec {
            width: self.width,
            height: self.height,
            fps: self.fps,
        }
    }

    /// Bytes of one RGB24 frame.
    pub fn rgb_len(&self) -> usize {
        self.width as usize * self.height as usize * 3
    }
}
