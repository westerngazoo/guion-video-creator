//! The automatic gate of a narrated reel (R-0012): everything about a
//! rendered reel that can be measured, measured against its brand's profile,
//! before any person reviews it.
//!
//! Inputs: the reel's MP4 with its audio, `voz.json` (the voice's timeline),
//! `eventos.json` (the clock), `caption.md`, and a [`Profile`]. Output: a
//! [`Report`], one [`Finding`] per check. One decode of the video serves the
//! motion scan (every frame) and the pixel scan (every `scan.stride` frames).
//!
//! What is *judgement* (a model's result said as a fact, a circular cause, a
//! drawing that is physically false) is not here: it stays with the
//! reviewers. What is here is what reviewers kept catching by hand and a
//! machine can count.

pub mod audio;
pub mod caption;
pub mod motion;
pub mod pixels;
pub mod profile;
pub mod report;
pub mod subtitles;
pub mod video;
pub mod youtube;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub use profile::Profile;
pub use report::{Finding, Report};

use crate::formato::Formato;
use crate::narration::Narration;
use pixels::{outside_safe, regions, set, Classifier};
use report::{at_frame, MAX_AT};

/// What can stop the gate from running at all (a FAIL is not an error: it
/// is a finding).
#[derive(Debug, Clone, PartialEq)]
pub enum GateError {
    /// The profile does not parse or does not hold together.
    Profile(String),
    /// An input is missing or unreadable (names the file).
    Input(String),
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateError::Profile(m) => write!(f, "brand profile: {m}"),
            GateError::Input(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for GateError {}

/// The files the gate reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Inputs {
    /// The post's folder (named in the report).
    pub post: PathBuf,
    /// The reel with its audio.
    pub mp4: PathBuf,
    pub voice: PathBuf,
    pub events: PathBuf,
    pub caption: PathBuf,
    /// Two renders of the same reel: same bytes or FAIL (determinism).
    pub compare: Option<(PathBuf, PathBuf)>,
    /// The longest the reel may last, s.
    pub max_duration: Option<f64>,
    /// The format it is checked as (R-0016): the profile's bands for it, its
    /// platform's length, and its texts (the caption for a reel or TikTok;
    /// the title and description for YouTube; the `.srt`, chapters and
    /// thumbnail for a long YouTube video).
    pub formato: Formato,
    /// The YouTube deliverables: subtitles, title, description, thumbnail.
    pub srt: PathBuf,
    pub title: PathBuf,
    pub description: PathBuf,
    pub thumbnail: PathBuf,
}

impl Inputs {
    /// The usual layout of a post folder: `<name>-audio.mp4`, `voz.json`,
    /// `eventos.json` and `caption.md`; for YouTube, `<name>.srt`,
    /// `titulo.txt`, `descripcion.txt` and `miniatura.png`. A reel.
    pub fn in_post(post: &Path, name: &str) -> Inputs {
        Inputs {
            post: post.to_path_buf(),
            mp4: post.join(format!("{name}-audio.mp4")),
            voice: post.join("voz.json"),
            events: post.join("eventos.json"),
            caption: post.join("caption.md"),
            compare: None,
            max_duration: None,
            formato: Formato::Reel,
            srt: post.join(format!("{name}.srt")),
            title: post.join("titulo.txt"),
            description: post.join("descripcion.txt"),
            thumbnail: post.join("miniatura.png"),
        }
    }

    /// The YouTube texts and thumbnail in `dir` (where they were exported).
    pub fn youtube_in(mut self, dir: &Path, name: &str) -> Inputs {
        self.srt = dir.join(format!("{name}.srt"));
        self.title = dir.join("titulo.txt");
        self.description = dir.join("descripcion.txt");
        self.thumbnail = dir.join("miniatura.png");
        self
    }
}

fn need(p: &Path) -> Result<(), GateError> {
    if p.exists() {
        Ok(())
    } else {
        Err(GateError::Input(format!("{}: not found", p.display())))
    }
}

/// Consecutive scanned frames that failed the same way, merged.
struct Misses {
    stride: usize,
    fps: f64,
    runs: Vec<(usize, usize, String, usize)>, // first, last, detail of the worst, its score
    frames: usize,
    /// Stretches dropped by [`Misses::lasting`].
    transient: usize,
}

impl Misses {
    fn new(stride: usize, fps: f64) -> Misses {
        Misses {
            stride,
            fps,
            runs: Vec::new(),
            frames: 0,
            transient: 0,
        }
    }

    fn add(&mut self, frame: usize, score: usize, detail: impl FnOnce() -> String) {
        self.frames += 1;
        if let Some(r) = self.runs.last_mut() {
            if frame - r.1 <= self.stride {
                r.1 = frame;
                if score > r.3 {
                    r.2 = detail();
                    r.3 = score;
                }
                return;
            }
        }
        self.runs.push((frame, frame, detail(), score));
    }

    fn at(&self) -> Vec<String> {
        self.runs
            .iter()
            .take(MAX_AT)
            .map(|(a, b, d, _)| {
                if a == b {
                    format!("{} {d}", at_frame(*a, self.fps))
                } else {
                    format!(
                        "{:.2}–{:.2} s (f{a}–f{b}) {d}",
                        *a as f64 / self.fps,
                        *b as f64 / self.fps
                    )
                }
            })
            .collect()
    }

    /// Drops the stretches that last less than `secs` (an entrance or an exit
    /// that sweeps through a margin in a frame or two).
    fn lasting(mut self, secs: f64) -> Misses {
        let (stride, fps) = (self.stride, self.fps);
        let before = self.runs.len();
        self.runs
            .retain(|(a, b, _, _)| (b - a + stride) as f64 / fps >= secs);
        self.transient = before - self.runs.len();
        self.frames = self
            .runs
            .iter()
            .map(|(a, b, _, _)| (b - a) / stride + 1)
            .sum();
        self
    }

    fn finding(&self, check: &str, ok: String, what: &str) -> Finding {
        Finding::from_misses(
            check,
            ok,
            format!(
                "{what} in {} scanned frames ({} stretches)",
                self.frames,
                self.runs.len()
            ),
            self.at(),
        )
    }
}

fn region_box(r: &pixels::Region) -> String {
    format!("[{}, {}, {}, {}]", r.x0, r.y0, r.x1 + 1, r.y1 + 1)
}

/// Runs every check, with the brand profile `p` as it applies to
/// `inputs.formato` ([`Profile::para`]). The report says PASS only if every
/// finding does.
pub fn run(inputs: &Inputs, p: &Profile) -> Result<Report, GateError> {
    let started = Instant::now();
    let formato = inputs.formato;
    let p = &p.para(formato);
    let caption = formato.con_caption() || inputs.caption.exists();
    for f in [&inputs.mp4, &inputs.voice, &inputs.events] {
        need(f)?;
    }
    if formato.con_caption() {
        need(&inputs.caption)?;
    }
    let narration = Narration::load(&inputs.voice).map_err(|e| GateError::Input(e.to_string()))?;
    let meter = subtitles::SubtitleMeter::new(p)?;
    let mut findings = subtitles::check(&narration, p, &meter)?;
    let widths: Vec<f64> = narration
        .lines
        .iter()
        .map(|l| meter.width(&l.subtitle))
        .collect::<Result<_, _>>()?;

    // --- the video: one decode, motion on every frame, pixels on every stride-th ---
    let (w, h) = video::size(&inputs.mp4)?;
    if (w, h) != (p.canvas.width as usize, p.canvas.height as usize) {
        return Err(GateError::Input(format!(
            "{} is {w}×{h}; the profile's canvas is {}×{}",
            inputs.mp4.display(),
            p.canvas.width,
            p.canvas.height
        )));
    }
    let fps = p.canvas.fps;
    let classes = Classifier::new(p);
    let red = set(&classes.ids(&p.roles.red));
    let focus = set(&classes.ids(&p.roles.focus));
    let background = set(&classes.ids(&p.roles.background));

    let mut ink = if p.safe.ink.is_empty() {
        let mut s = [false; 256];
        for i in 0..classes.names.len() {
            s[i] = !background[i];
        }
        s
    } else {
        set(&classes.ids(&p.safe.ink))
    };
    ink[pixels::NONE as usize] = false;
    let formula = p
        .formula
        .as_ref()
        .map(|f| (f.band, f.max_height, set(&classes.ids(&f.tokens))));
    let band = (
        p.subtitle.band[0].max(0.0) as usize,
        (p.subtitle.band[1].max(0.0) as usize).min(h),
    );
    let (merge, min_area, stride) = (p.scan.merge, p.scan.min_area, p.scan.stride);

    let mut frames = video::Frames::open(&inputs.mp4, w, h)?;
    let (mut rgb, mut class) = (Vec::new(), Vec::new());
    // `anchor` is the last frame that moved: a slow fade adds up against it
    let (mut blocks, mut anchor) = (Vec::new(), Vec::new());
    let mut moving = Vec::new();
    let mut m_red = Misses::new(stride, fps);
    let mut m_focus = Misses::new(stride, fps);
    let mut m_safe = Misses::new(stride, fps);
    let mut m_band = Misses::new(stride, fps);
    let mut m_formula = Misses::new(stride, fps);
    let (mut max_red, mut max_focus) = (0, 0);
    let mut scanned = 0;
    let mut i = 0;
    while frames.next_into(&mut rgb)? {
        motion::block_means(&rgb, w, h, band, &mut blocks);
        let m = i == 0 || motion::moved(&blocks, &anchor, &p.motion);
        moving.push(m);
        if m {
            std::mem::swap(&mut blocks, &mut anchor);
        }
        if i % stride == 0 {
            scanned += 1;
            classes.classify(&rgb, &mut class);
            let reds = regions(&class, w, h, &red, merge, min_area, None);
            max_red = max_red.max(reds.len());
            if reds.len() > 1 {
                m_red.add(i, reds.len(), || {
                    format!(
                        "{} regions: {}",
                        reds.len(),
                        reds.iter().map(region_box).collect::<Vec<_>>().join(" ")
                    )
                });
            }
            if !p.roles.focus.is_empty() {
                let fs = regions(
                    &class,
                    w,
                    h,
                    &focus,
                    merge,
                    min_area.max(p.scan.focus_min_area),
                    None,
                );
                max_focus = max_focus.max(fs.len());
                if fs.len() > 1 {
                    m_focus.add(i, fs.len(), || {
                        format!(
                            "{} regions: {}",
                            fs.len(),
                            fs.iter().map(region_box).collect::<Vec<_>>().join(" ")
                        )
                    });
                }
            }
            let (n, bb) = outside_safe(&class, w, h, &ink, p);
            if n >= p.scan.min_stray {
                m_safe.add(i, n, || {
                    format!(
                        "{n} px in {}",
                        bb.as_ref().map(region_box).unwrap_or_default()
                    )
                });
            }
            // the subtitle band: only the subtitle, where and when it shows
            let t = i as f64 / fps;
            let extent = narration.active(t, p.subtitle.fade).map(|(l, _)| {
                let k = narration
                    .lines
                    .iter()
                    .position(|x| std::ptr::eq(x, l))
                    .unwrap_or(0);
                let half = widths[k] / 2.0 + 16.0;
                (p.subtitle.center_x - half, p.subtitle.center_x + half)
            });
            let mut foreign = 0;
            let mut fb = (usize::MAX, usize::MAX, 0, 0);
            for y in band.0..band.1 {
                for x in 0..w {
                    let c = class[y * w + x] as usize;
                    if c == pixels::NONE as usize || background[c] {
                        continue;
                    }
                    // inside the box the subtitle is measured to fill, everything is the
                    // subtitle: its anti-aliased edges, after 4:2:0, match no clean blend
                    let ok = extent.is_some_and(|(a, b)| (x as f64) >= a && (x as f64) <= b);
                    if !ok {
                        foreign += 1;
                        fb = (fb.0.min(x), fb.1.min(y), fb.2.max(x), fb.3.max(y));
                    }
                }
            }
            if foreign >= p.scan.min_stray {
                m_band.add(i, foreign, || {
                    format!(
                        "{foreign} px in [{}, {}, {}, {}]",
                        fb.0,
                        fb.1,
                        fb.2 + 1,
                        fb.3 + 1
                    )
                });
            }
            if let Some((fband, tallest, fset)) = &formula {
                let rs = regions(&class, w, h, fset, merge, min_area, Some(band));
                let (b0, b1) = (fband[0], fband[1]);
                let out: Vec<&pixels::Region> = rs
                    .iter()
                    .filter(|r| (r.y1 as f64) >= b0 && (r.y0 as f64) < b1)
                    .filter(|r| ((r.y1 - r.y0 + 1) as f64) <= *tallest)
                    .filter(|r| (r.y0 as f64) < b0 || (r.y1 as f64) >= b1)
                    .collect();
                if !out.is_empty() {
                    m_formula.add(i, out.len(), || {
                        out.iter()
                            .map(|r| region_box(r))
                            .collect::<Vec<_>>()
                            .join(" ")
                    });
                }
            }
        }
        i += 1;
    }
    frames.finish()?;
    let n = moving.len();

    findings.push(m_red.finding(
        "red",
        format!("≤ 1 red region in {scanned} scanned frames (max {max_red})"),
        "two red regions or more",
    ));
    if !p.roles.focus.is_empty() {
        findings.push(m_focus.finding(
            "focus",
            format!("≤ 1 focus region in {scanned} scanned frames (max {max_focus})"),
            "two focus regions or more",
        ));
    }
    let m_safe = m_safe.lasting(p.safe.min_duration);
    let passing = if m_safe.transient > 0 {
        format!(
            "nothing outside the safe area ({} passing sweeps under {:.1} s)",
            m_safe.transient, p.safe.min_duration
        )
    } else {
        "nothing outside the safe area".into()
    };
    findings.push(m_safe.finding("safe-area", passing, "ink outside the safe area"));
    let m_band = m_band.lasting(p.safe.min_duration);
    let passing = if m_band.transient > 0 {
        format!(
            "only the subtitle in its band ({} passing sweeps under {:.1} s)",
            m_band.transient, p.safe.min_duration
        )
    } else {
        "only the subtitle in its band".into()
    };
    findings.push(m_band.finding("sub-band", passing, "something else in the subtitle band"));
    if let Some(f) = &p.formula {
        findings.push(m_formula.finding(
            "formula-band",
            format!("formulas inside y {:.0}–{:.0}", f.band[0], f.band[1]),
            "a formula crossing its band",
        ));
    }
    let voice: Vec<(f64, f64)> = narration.lines.iter().map(|l| (l.t0, l.t1)).collect();
    findings.extend(motion::check(&moving, &voice, p));

    // --- the clock and the length (the post's, the brand's and the platform's) ---
    let max = match (inputs.max_duration, p.max_duration(formato)) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    findings.push(clock(&inputs.events, max, n, fps)?);

    // --- the audio ---
    findings.push(audio::loudness(&inputs.mp4, p)?);
    findings.push(audio::pumping(
        &inputs.mp4,
        &narration,
        &events(&inputs.events)?,
        p,
    )?);

    // --- the caption ---
    if caption {
        findings.extend(caption::check(&inputs.caption, p));
    }

    // --- YouTube (R-0016) ---
    if formato.de_youtube() {
        findings.extend(youtube::texts(&inputs.title, &inputs.description, p));
    }
    if formato == Formato::Youtube {
        findings.push(youtube::srt(&inputs.srt, &narration));
        findings.push(youtube::chapters(&inputs.description, n as f64 / fps));
        findings.push(youtube::thumbnail(&inputs.thumbnail));
    }

    // --- determinism ---
    findings.push(determinism(inputs.compare.as_ref())?);

    let name = inputs.post.display().to_string();
    Ok(Report::new(name, findings, started.elapsed().as_secs_f64()))
}

/// Two renders of one reel: the same bytes, or FAIL (R-0012 AC9).
pub fn determinism(compare: Option<&(PathBuf, PathBuf)>) -> Result<Finding, GateError> {
    Ok(match compare {
        None => Finding::pass("determinism", "not run (no second render given)"),
        Some((one, other)) => {
            let read = |p: &PathBuf| {
                std::fs::read(p).map_err(|e| GateError::Input(format!("{}: {e}", p.display())))
            };
            let (a, b) = (read(one)?, read(other)?);
            if a == b {
                Finding::pass(
                    "determinism",
                    format!("two renders, same {} bytes", a.len()),
                )
            } else {
                Finding::fail(
                    "determinism",
                    format!("two renders differ ({} vs {} bytes)", a.len(), b.len()),
                    vec![other.display().to_string()],
                )
            }
        }
    })
}

/// The named instants of `eventos.json`.
fn events(path: &Path) -> Result<Vec<(String, f64)>, GateError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| GateError::Input(format!("{}: {e}", path.display())))?;
    let v: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| GateError::Input(format!("{}: {e}", path.display())))?;
    Ok(v.get("eventos")
        .and_then(|e| e.as_object())
        .map(|o| {
            o.iter()
                .filter_map(|(k, t)| t.as_f64().map(|t| (k.clone(), t)))
                .collect()
        })
        .unwrap_or_default())
}

/// The clock: the MP4 has the frames `eventos.json` says, at the profile's
/// fps, and lasts no longer than allowed.
fn clock(
    events: &Path,
    max_duration: Option<f64>,
    frames: usize,
    fps: f64,
) -> Result<Finding, GateError> {
    #[derive(serde::Deserialize)]
    struct Head {
        duracion: f64,
        fps: f64,
    }
    let text = std::fs::read_to_string(events)
        .map_err(|e| GateError::Input(format!("{}: {e}", events.display())))?;
    let head: Head = serde_json::from_str(&text)
        .map_err(|e| GateError::Input(format!("{}: {e}", events.display())))?;
    let expected = (head.duracion * head.fps).round() as usize;
    let secs = frames as f64 / fps;
    let mut bad = Vec::new();
    if (head.fps - fps).abs() > 1e-9 {
        bad.push(format!(
            "eventos.json says {} fps, the profile {fps}",
            head.fps
        ));
    }
    if expected != frames {
        bad.push(format!(
            "eventos.json says {expected} frames, the MP4 has {frames}"
        ));
    }
    if let Some(max) = max_duration {
        if secs > max {
            bad.push(format!("{secs:.2} s > {max:.1} s"));
        }
    }
    let limit = max_duration
        .map(|m| format!(" ≤ {m:.1}"))
        .unwrap_or_default();
    Ok(Finding::from_misses(
        "duration",
        format!("{frames} frames, {secs:.2} s{limit}"),
        "the clock and the MP4 disagree",
        bad,
    ))
}
