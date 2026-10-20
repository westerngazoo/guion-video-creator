//! The brand profile the gate measures against (R-0012 AC1).
//!
//! Every value the gate compares with comes from here: the palette tokens and
//! the role each one plays, the subtitle's face, size and box, the bands, the
//! safe area, the motion and audio limits, and the caption's rules. The crate
//! holds none of them (R-0009 AC7 still holds): a brand writes one TOML and
//! every reel of that brand is checked the same way.
//!
//! **Formats (R-0016).** The top-level `[canvas]`, `[subtitle]`, `[safe]` and
//! `[formula]` describe the brand's own format (a 9:16 reel, for the house's
//! brands). [`Profile::para`] gives the profile a format is checked with: the
//! same one for a format of the same orientation, the house's default bands
//! ([`crate::Formato::bandas`]) for the other orientation, and on top of either
//! what the brand writes under `[formato.<name>]`:
//!
//! ```toml
//! [formato.youtube]
//! canvas = { width = 1920, height = 1080, fps = 30.0 }
//! subtitle = { band = [940.0, 1025.0], baseline = 980.0, center_x = 960.0, max_width = 1400.0 }
//! safe = { x = [96.0, 1824.0], y = [54.0, 1025.0], except = [[1700.0, 60.0, 1860.0, 140.0]] }
//! formula_band = [700.0, 900.0]
//! max_duration = 900.0
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::GateError;
use crate::formato::Formato;

/// An RGB colour, `[r, g, b]` (an `[r, g, b, a]` token keeps only its colour:
/// the gate never classifies translucent fills).
pub type Rgb = [u8; 3];

/// A brand profile, as loaded from its TOML.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub brand: BrandInfo,
    pub canvas: CanvasSpec,
    /// Every opaque token of the palette, by name.
    #[serde(deserialize_with = "tokens")]
    pub tokens: BTreeMap<String, Rgb>,
    pub roles: Roles,
    pub subtitle: SubtitleSpec,
    #[serde(default)]
    pub formula: Option<FormulaSpec>,
    pub safe: SafeArea,
    pub motion: MotionSpec,
    pub scan: ScanSpec,
    pub audio: AudioSpec,
    pub caption: CaptionSpec,
    /// Per-format overrides, by format name (R-0016).
    #[serde(default)]
    pub formato: BTreeMap<String, FormatoSpec>,
    /// Where the profile was read from: its relative font paths resolve here.
    #[serde(skip)]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrandInfo {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasSpec {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}

/// What each token is for. Every name must be a key of `[tokens]`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    /// What counts as empty: the background and its facets or cards.
    pub background: Vec<String>,
    /// "The" accent: at most one region of these per frame.
    pub red: Vec<String>,
    /// The focus colour: at most one region per frame. Empty: not checked
    /// (a brand whose gold is everywhere, by design).
    #[serde(default)]
    pub focus: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleSpec {
    /// The face the subtitle is set in (a TTF/OTF, relative to the profile).
    pub font: String,
    /// The size, px, in `size_unit`.
    pub size: f64,
    /// `"em"` (CSS-like: the em is `size` px) or `"line"` (motoreel's scale:
    /// the line height is `size` px).
    pub size_unit: SizeUnit,
    /// The face formula words are set in, and its scale; when absent,
    /// formula words are measured in `font`.
    #[serde(default)]
    pub formula_font: Option<String>,
    #[serde(default = "one")]
    pub formula_scale: f64,
    /// The breath a formula word gets, in total, as a fraction of `size`.
    #[serde(default)]
    pub formula_pad: f64,
    /// Whether a subtitle may carry formula words at all (guion-reel's
    /// [`crate::subtitle::is_formula_word`]).
    #[serde(default)]
    pub formula_words: bool,
    pub max_width: f64,
    pub center_x: f64,
    pub baseline: f64,
    /// The rows reserved for the subtitle: `[y0, y1]`, px.
    pub band: [f64; 2],
    pub max_words: usize,
    /// Characters a subtitle may not carry (signs that belong on screen).
    #[serde(default)]
    pub forbidden: String,
    /// Whether digits are forbidden.
    #[serde(default)]
    pub forbid_digits: bool,
    /// A line longer than this (s), with more than one sentence, must enter
    /// its subtitle on a later sentence (`desde`).
    pub long_line: f64,
    /// The fade in and out, s.
    pub fade: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeUnit {
    Em,
    Line,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormulaSpec {
    /// The formula band, `[y0, y1]`, px.
    pub band: [f64; 2],
    /// The tokens formulas are drawn in.
    pub tokens: Vec<String>,
    /// A region of those tokens taller than this (px) is a figure, not a
    /// formula (a brand whose mark shares the formula's gold).
    #[serde(default = "tall")]
    pub max_height: f64,
}

fn tall() -> f64 {
    f64::INFINITY
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeArea {
    /// `[x0, x1]` and `[y0, y1]`: ink outside fails.
    pub x: [f64; 2],
    pub y: [f64; 2],
    /// More rectangles that must stay empty, `[x0, y0, x1, y1]` (e.g. the
    /// platform's buttons on the right, below some height).
    #[serde(default)]
    pub free: Vec<[f64; 4]>,
    /// Rectangles exempt from the above (the watermark, a level chip),
    /// `[x0, y0, x1, y1]`.
    #[serde(default)]
    pub except: Vec<[f64; 4]>,
    /// The tokens that count as ink; empty: every token that is not background.
    #[serde(default)]
    pub ink: Vec<String>,
    /// Ink outside for less than this (s) is an entrance or an exit passing
    /// through, not a layout: it is reported, not failed.
    #[serde(default = "sweep")]
    pub min_duration: f64,
    /// How far (px) anti-aliasing and the codec may bleed an edge past a
    /// limit (the safe area, an exempt rectangle).
    #[serde(default = "bleed")]
    pub bleed: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotionSpec {
    /// The longest still stretch allowed while the voice speaks, s.
    pub max_still_with_voice: f64,
    /// The shortest closing hold (the last moving frame to the end), s.
    pub closing_hold: f64,
    /// A frame moved if one 8 × 8 block's mean luma changed more than
    /// `strong` (0–255)…
    #[serde(default = "strong")]
    pub strong: f32,
    /// …or at least `min_blocks` blocks changed more than `delta`, as long
    /// as one of them changed more than `noise`.
    #[serde(default = "delta")]
    pub delta: f32,
    #[serde(default = "min_blocks")]
    pub min_blocks: usize,
    /// The codec's noise floor in block-mean levels (0–255): how far H.264
    /// re-quantization (P-frame drift, a keyframe) moves the 8 × 8 means of
    /// a still picture. A change no block of which passes it is the codec.
    #[serde(default = "noise")]
    pub noise: f32,
}

fn strong() -> f32 {
    24.0
}
fn delta() -> f32 {
    4.0
}
fn min_blocks() -> usize {
    30
}
fn noise() -> f32 {
    8.0
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanSpec {
    /// Every how many frames the pixel scan looks.
    pub stride: usize,
    /// How far (Euclidean, 0–441) a decoded pixel may be from its token.
    pub tolerance: f64,
    /// Pixels closer than this (px) belong to one region.
    pub merge: usize,
    /// A region smaller than this (pixels of its token) is not counted.
    pub min_area: usize,
    /// A focus region counts only from this many pixels: the formula or the
    /// figure that is *the* focus, not the small labels named around it.
    #[serde(default)]
    pub focus_min_area: usize,
    /// Fewer stray ink pixels than this, in a zone, are codec noise.
    #[serde(default = "stray")]
    pub min_stray: usize,
}

fn sweep() -> f64 {
    0.3
}

fn bleed() -> f64 {
    2.0
}

fn stray() -> usize {
    40
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSpec {
    pub lufs: f64,
    pub tolerance: f64,
    pub true_peak: f64,
    /// A gap between two lines shorter than this (s) must not let the bed up…
    pub short_gap: f64,
    /// …measured as the gap's level relative to the lines around it, dB.
    pub gap_max_db: f64,
    /// Events (by name prefix, from `eventos.json`) where the bed may come
    /// up on purpose: a gap holding one (a beat change) is not pumping.
    #[serde(default)]
    pub beat_events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptionSpec {
    pub max_hashtags: usize,
    pub max_chars: usize,
    /// Units (and `%`) that need U+202F between the number and them.
    pub units: Vec<String>,
    /// Text that must never be published: literal, with `#` standing for
    /// one ASCII digit (`R-####`).
    pub internal: Vec<String>,
}

/// What a brand overrides for one format (R-0016); every key is optional.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatoSpec {
    #[serde(default)]
    pub canvas: Option<CanvasSpec>,
    #[serde(default)]
    pub subtitle: Option<SubtitleBands>,
    #[serde(default)]
    pub safe: Option<SafeBands>,
    /// The formula band in this format (the brand's `[formula]` tokens).
    #[serde(default)]
    pub formula_band: Option<[f64; 2]>,
    /// The longest the video may last here, s (never past the platform's).
    #[serde(default)]
    pub max_duration: Option<f64>,
}

/// The subtitle's geometry in one format.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleBands {
    #[serde(default)]
    pub band: Option<[f64; 2]>,
    #[serde(default)]
    pub baseline: Option<f64>,
    #[serde(default)]
    pub center_x: Option<f64>,
    #[serde(default)]
    pub max_width: Option<f64>,
}

/// The safe area in one format.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeBands {
    #[serde(default)]
    pub x: Option<[f64; 2]>,
    #[serde(default)]
    pub y: Option<[f64; 2]>,
    #[serde(default)]
    pub free: Option<Vec<[f64; 4]>>,
    #[serde(default)]
    pub except: Option<Vec<[f64; 4]>>,
}

fn tokens<'de, D>(d: D) -> Result<BTreeMap<String, Rgb>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: BTreeMap<String, Vec<u8>> = BTreeMap::deserialize(d)?;
    raw.into_iter()
        .map(|(k, v)| match v.as_slice() {
            [r, g, b] | [r, g, b, _] => Ok((k, [*r, *g, *b])),
            _ => Err(serde::de::Error::custom(format!(
                "token «{k}» must be [r, g, b] or [r, g, b, a]"
            ))),
        })
        .collect()
}

impl Profile {
    /// Reads and validates a profile file.
    pub fn load(path: &Path) -> Result<Profile, GateError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| GateError::Input(format!("{}: {e}", path.display())))?;
        let mut p = Profile::parse(&text).map_err(|e| match e {
            GateError::Profile(m) => GateError::Profile(format!("{}: {m}", path.display())),
            o => o,
        })?;
        p.dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Ok(p)
    }

    /// Parses a profile and checks that every role names a token.
    pub fn parse(text: &str) -> Result<Profile, GateError> {
        let p: Profile = toml::from_str(text).map_err(|e| GateError::Profile(e.to_string()))?;
        let named = |list: &[String], where_: &str| -> Result<(), GateError> {
            match list.iter().find(|n| !p.tokens.contains_key(*n)) {
                Some(n) => Err(GateError::Profile(format!(
                    "{where_} names «{n}», which is not in [tokens]"
                ))),
                None => Ok(()),
            }
        };
        named(&p.roles.background, "roles.background")?;
        named(&p.roles.red, "roles.red")?;
        named(&p.roles.focus, "roles.focus")?;
        named(&p.safe.ink, "safe.ink")?;
        if let Some(f) = &p.formula {
            named(&f.tokens, "formula.tokens")?;
        }
        if p.roles.background.is_empty() || p.roles.red.is_empty() {
            return Err(GateError::Profile(
                "roles.background and roles.red need at least one token".into(),
            ));
        }
        if p.scan.stride == 0 || p.canvas.fps <= 0.0 || !p.canvas.fps.is_finite() {
            return Err(GateError::Profile(
                "scan.stride and canvas.fps must be > 0".into(),
            ));
        }
        for (name, o) in &p.formato {
            Formato::de(name).map_err(|e| GateError::Profile(format!("[formato.{name}]: {e}")))?;
            if o.formula_band.is_some() && p.formula.is_none() {
                return Err(GateError::Profile(format!(
                    "formato.{name}.formula_band needs a [formula] (its tokens)"
                )));
            }
            if let Some(c) = &o.canvas {
                if c.width == 0 || c.height == 0 || c.fps <= 0.0 || !c.fps.is_finite() {
                    return Err(GateError::Profile(format!(
                        "formato.{name}.canvas: width, height and fps must be > 0"
                    )));
                }
            }
        }
        Ok(p)
    }

    /// The profile format `f` is checked with (R-0016): this one when `f`
    /// has the orientation of `[canvas]`, the house's default bands for `f`
    /// otherwise, then the brand's `[formato.<f>]` on top. A brand that
    /// writes nothing for a format of its own orientation gets itself back,
    /// unchanged.
    pub fn para(&self, f: Formato) -> Profile {
        let mut q = self.clone();
        let vertical = self.canvas.height > self.canvas.width;
        if f.vertical() != vertical {
            let b = f.bandas();
            q.canvas.width = b.width;
            q.canvas.height = b.height;
            q.subtitle.band = b.subtitle_band;
            q.subtitle.baseline = b.baseline;
            q.subtitle.center_x = b.center_x;
            q.subtitle.max_width = b.max_width;
            q.safe.x = b.safe_x;
            q.safe.y = b.safe_y;
            q.safe.free = b.free;
            q.safe.except = b.except;
            q.formula = match (q.formula.take(), b.formula_band) {
                (Some(mut fs), Some(band)) => {
                    fs.band = band;
                    Some(fs)
                }
                // a formula band is the brand's design in its own format
                _ => None,
            };
        }
        let Some(o) = self.formato.get(f.nombre()) else {
            return q;
        };
        if let Some(c) = &o.canvas {
            q.canvas = c.clone();
        }
        if let Some(s) = &o.subtitle {
            q.subtitle.band = s.band.unwrap_or(q.subtitle.band);
            q.subtitle.baseline = s.baseline.unwrap_or(q.subtitle.baseline);
            q.subtitle.center_x = s.center_x.unwrap_or(q.subtitle.center_x);
            q.subtitle.max_width = s.max_width.unwrap_or(q.subtitle.max_width);
        }
        if let Some(s) = &o.safe {
            q.safe.x = s.x.unwrap_or(q.safe.x);
            q.safe.y = s.y.unwrap_or(q.safe.y);
            if let Some(v) = &s.free {
                q.safe.free = v.clone();
            }
            if let Some(v) = &s.except {
                q.safe.except = v.clone();
            }
        }
        if let Some(band) = o.formula_band {
            // the brand's formula tokens, in this format's band
            q.formula = q
                .formula
                .take()
                .or_else(|| self.formula.clone())
                .map(|mut fs| {
                    fs.band = band;
                    fs
                });
        }
        q
    }

    /// The longest a video of format `f` may last, s: the brand's
    /// `[formato.<f>] max_duration` and the platform's, the shorter.
    pub fn max_duration(&self, f: Formato) -> Option<f64> {
        let brand = self.formato.get(f.nombre()).and_then(|o| o.max_duration);
        match (brand, f.duracion_maxima()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// A path from the profile, resolved against the profile's folder.
    pub fn resolve(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}
