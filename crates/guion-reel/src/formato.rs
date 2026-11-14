//! The formats one post can be delivered in (R-0016): the canvas, the
//! platform's limits and the default bands each one is checked with.
//!
//! | Format    | Canvas              | Platform limit (reported)                     |
//! |-----------|---------------------|-----------------------------------------------|
//! | `reel`    | 9:16, 1080 × 1920   | — (the post's `max_duration`)                 |
//! | `tiktok`  | 9:16, 1080 × 1920   | — (the same MP4 as the reel)                  |
//! | `short`   | 9:16, 1080 × 1920   | ≤ 3 min ([`SHORT_MAX_S`])                     |
//! | `youtube` | 16:9, 1920 × 1080   | chapters, thumbnail, title, description, .srt |
//!
//! The vertical formats also carry where a cover's content must sit
//! ([`Encuadre`], R-0017): Instagram crops a 9:16 reel in the profile grid
//! and in the feed, and its own UI covers the bottom of the Reels viewer.
//!
//! The format is the post's (`formato` in `reel.toml` or `escena.toml`); a
//! scene binary run by `guion reel` reads it from [`VARIABLE`]
//! ([`Formato::desde_entorno`]). Nothing here belongs to a brand: the bands
//! are the house's defaults, and an account overrides them in its gate
//! profile (`[formato.<name>]`, see [`crate::gate::profile`]).

use crate::canvas::Canvas;

/// The environment variable `guion reel` sets for the scene binary and the
/// sound command: the format being rendered.
pub const VARIABLE: &str = "GUION_FORMATO";

/// The longest a YouTube Short may last, s: «up to three minutes» for uploads
/// on or after 2024-10-15 (reported: YouTube Help, «Understand three-minute
/// YouTube Shorts», support.google.com/youtube/answer/15424877).
pub const SHORT_MAX_S: f64 = 180.0;

/// A delivery format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Formato {
    /// An Instagram reel: 9:16.
    Reel,
    /// TikTok: the reel's canvas and MP4.
    Tiktok,
    /// A YouTube Short: 9:16, up to three minutes.
    Short,
    /// A long YouTube video: 16:9.
    Youtube,
}

/// The geometry the gate checks a format with: the canvas size, the
/// subtitle's row, the safe area and the formula band, in that canvas's
/// pixels (y down).
#[derive(Debug, Clone, PartialEq)]
pub struct Bandas {
    pub width: u32,
    pub height: u32,
    /// The subtitle's rows `[y0, y1]`, its baseline, centre and widest row.
    pub subtitle_band: [f64; 2],
    pub baseline: f64,
    pub center_x: f64,
    pub max_width: f64,
    /// The safe area `[x0, x1]`, `[y0, y1]`, and its free and exempt
    /// rectangles `[x0, y0, x1, y1]`.
    pub safe_x: [f64; 2],
    pub safe_y: [f64; 2],
    pub free: Vec<[f64; 4]>,
    pub except: Vec<[f64; 4]>,
    /// The formula band, when the format has one.
    pub formula_band: Option<[f64; 2]>,
    /// Where the cover's content must sit (R-0017); `None`: not checked
    /// (a horizontal format is never cropped to a grid tile).
    pub encuadre: Option<Encuadre>,
}

/// Where the content of a vertical cover must sit, px of a 1080 × 1920
/// canvas, y down (R-0017). The gate measures the box of the cover frame's
/// ink and fails when its top is above `min_top`, its bottom below
/// `max_bottom`, or its centre further than `max_offset` from `center_y`.
///
/// What Instagram does with a 9:16 reel is *reportado* by third parties, not
/// documented by Meta (R-0017 §2): the profile grid shows a centred 3:4 crop
/// (1080 × 1440, y 240–1680), the main feed a centred 4:5 crop (1080 × 1350,
/// y 285–1635), and the Reels viewer covers about the bottom 380 px with its
/// UI. Sources: <https://posteverywhere.ai/blog/instagram-reels-aspect-ratio>,
/// <https://moda.app/resources/sizes/instagram-reel>,
/// <https://admakeai.com/blog/instagram-reel-size>.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encuadre {
    /// The highest the content may start: the 4:5 feed crop's top (285)
    /// plus a 15 px margin.
    pub min_top: f64,
    /// The lowest the content may end: above the Reels UI (1920 − 380 ≈
    /// 1540), on the subtitle band's bottom edge (1545).
    pub max_bottom: f64,
    /// The row the content's box is centred on: the 3:4 grid crop's centre
    /// (and the canvas's).
    pub center_y: f64,
    /// How far, px, the box's centre may sit from `center_y`.
    pub max_offset: f64,
}

/// The house's cover framing for 1080 × 1920 (R-0017 AC1).
pub const ENCUADRE_VERTICAL: Encuadre = Encuadre {
    min_top: 300.0,
    max_bottom: 1545.0,
    center_y: 960.0,
    max_offset: 60.0,
};

impl Formato {
    /// Every format, in the order they are listed.
    pub const TODOS: [Formato; 4] = [
        Formato::Reel,
        Formato::Tiktok,
        Formato::Short,
        Formato::Youtube,
    ];

    /// The name written in `reel.toml`, `escena.toml` and `--formato`.
    pub fn nombre(self) -> &'static str {
        match self {
            Formato::Reel => "reel",
            Formato::Tiktok => "tiktok",
            Formato::Short => "short",
            Formato::Youtube => "youtube",
        }
    }

    /// A format by name; the error lists the valid ones.
    pub fn de(s: &str) -> Result<Formato, String> {
        Formato::TODOS
            .into_iter()
            .find(|f| f.nombre() == s.trim())
            .ok_or_else(|| {
                format!(
                    "formato «{s}»: one of {}",
                    Formato::TODOS.map(Formato::nombre).join(", ")
                )
            })
    }

    /// The format `guion reel` asked for ([`VARIABLE`]), or the reel when it
    /// is unset: a scene binary run by hand renders what it always did.
    pub fn desde_entorno() -> Result<Formato, String> {
        match std::env::var(VARIABLE) {
            Ok(v) if !v.trim().is_empty() => Formato::de(&v),
            _ => Ok(Formato::Reel),
        }
    }

    /// The canvas: 9:16 at 1080 × 1920, or 16:9 at 1920 × 1080, 30 fps,
    /// 600 px per view unit in both (the short side is 1.8 units).
    pub fn canvas(self) -> Canvas {
        match self {
            Formato::Youtube => Canvas::HORIZONTAL_VIDEO,
            _ => Canvas::VERTICAL_REEL,
        }
    }

    /// Whether the canvas is portrait.
    pub fn vertical(self) -> bool {
        self != Formato::Youtube
    }

    /// Whether it is uploaded to YouTube (a title and a description).
    pub fn de_youtube(self) -> bool {
        matches!(self, Formato::Short | Formato::Youtube)
    }

    /// Whether it carries an Instagram/TikTok caption (`caption.md`).
    pub fn con_caption(self) -> bool {
        matches!(self, Formato::Reel | Formato::Tiktok)
    }

    /// The platform's longest length, s, if it has one this crate checks.
    pub fn duracion_maxima(self) -> Option<f64> {
        match self {
            Formato::Short => Some(SHORT_MAX_S),
            _ => None,
        }
    }

    /// The house's default bands (supuestos, not platform values): what the
    /// gate checks a format with when the account's profile does not say.
    /// 9:16 keeps the margins the first studio measured (Rotor's §6 and §12:
    /// 90 px at the sides, the subtitle on 1500) with the top at 300, not 250:
    /// ink above 300 is outside Instagram's 4:5 feed crop, which the cover
    /// framing ([`Encuadre`], R-0017) fails anyway; 16:9 keeps 5 % at the
    /// sides and the top and sets the subtitle on y = 980.
    pub fn bandas(self) -> Bandas {
        if self.vertical() {
            Bandas {
                width: 1080,
                height: 1920,
                subtitle_band: [1460.0, 1545.0],
                baseline: 1500.0,
                center_x: 540.0,
                max_width: 840.0,
                safe_x: [90.0, 990.0],
                safe_y: [300.0, 1545.0],
                free: Vec::new(),
                except: Vec::new(),
                formula_band: None,
                encuadre: Some(ENCUADRE_VERTICAL),
            }
        } else {
            Bandas {
                width: 1920,
                height: 1080,
                subtitle_band: [940.0, 1025.0],
                baseline: 980.0,
                center_x: 960.0,
                max_width: 1400.0,
                safe_x: [96.0, 1824.0],
                safe_y: [54.0, 1025.0],
                free: Vec::new(),
                except: Vec::new(),
                formula_band: None,
                encuadre: None,
            }
        }
    }
}

impl std::fmt::Display for Formato {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.nombre())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_an_unknown_one_lists_the_valid() {
        for f in Formato::TODOS {
            assert_eq!(Formato::de(f.nombre()).unwrap(), f);
        }
        let e = Formato::de("vertical").unwrap_err();
        assert!(e.contains("reel, tiktok, short, youtube"), "{e}");
    }

    #[test]
    fn the_canvas_follows_the_format() {
        assert_eq!(Formato::Reel.canvas(), Canvas::VERTICAL_REEL);
        assert_eq!(Formato::Short.canvas(), Canvas::VERTICAL_REEL);
        let c = Formato::Youtube.canvas();
        assert_eq!((c.width, c.height, c.px_per_unit), (1920, 1080, 600.0));
        let b = Formato::Youtube.bandas();
        assert!(b.subtitle_band[1] <= b.height as f64 && b.safe_x[1] <= b.width as f64);
    }
}
