//! The studio: everything a reel needs around its pure `frame(t)` (AC2–AC4).
//!
//! A scene is a function of time that returns motoreel primitives (and,
//! optionally, raster overlays to compose on top). [`Reel`] turns it into:
//!
//! - **the MP4**, in parallel ([`Reel::video`]): frame *i* is drawn by thread
//!   *i* mod *n*, each thread with its own sink, and ffmpeg receives the
//!   frames in order through [`guion_encode::encode_ordered_with`]. Each frame
//!   is a pure function of `t = i / fps` and the sink clears its canvas every
//!   frame, so the file is byte-identical with one thread or *n* (AC2);
//! - **review frames** ([`Reel::frames_png`]): loose PNGs at chosen times;
//! - **the contact sheet** ([`Reel::contact_sheet`]): one image with the key
//!   frames, each labelled with its time and the event it follows (AC3);
//! - **`events.json`** ([`Reel::write_events`]): the clock (AC1).
//!
//! [`Command`] reads the same choices from a command line.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};

use guion_encode::encode_ordered_with;
use motoreel::shapes::polygon;
use motoreel::{Align, Fill, FrameSink, PpmSink, Prim2, Rgb, Style};
use motoreel_typeset::Fonts;

use crate::canvas::Canvas;
use crate::clock::Clock;
use crate::error::ReelError;

/// A raster layer composed onto a rendered frame, after motoreel draws its
/// primitives and before the frame is encoded (AC4). The studio's own logo
/// files, which must never be redrawn, go in this way.
pub trait Overlay {
    /// Composes this layer onto a full binary PPM (P6 header included).
    fn compose(&self, p6: &mut [u8]) -> std::io::Result<()>;
}

/// The overlay type of a reel that has none.
pub enum NoOverlay {}

impl Overlay for NoOverlay {
    fn compose(&self, _: &mut [u8]) -> std::io::Result<()> {
        match *self {}
    }
}

/// One frame: its primitives and its overlays, composed in order.
pub type Frame<L> = (Vec<Prim2>, Vec<L>);

/// Where the studio writes, relative to the reel's folder.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The clock file.
    pub events: String,
    /// The folder for review frames and the contact sheet.
    pub review: String,
    /// The contact sheet's file name, inside `review`.
    pub sheet: String,
}

impl Default for Layout {
    fn default() -> Layout {
        Layout {
            events: "events.json".into(),
            review: "frames".into(),
            sheet: "sheet.png".into(),
        }
    }
}

/// The contact sheet's grid: `columns` cells per row, each the frame reduced
/// `reduction` times (by averaging each block), `gap` px apart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetGrid {
    pub columns: usize,
    pub reduction: usize,
    pub gap: usize,
}

impl Default for SheetGrid {
    fn default() -> SheetGrid {
        SheetGrid {
            columns: 4,
            reduction: 4,
            gap: 6,
        }
    }
}

/// What to do, as read from a command line ([`Command::from_args`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// The MP4 (and `events.json`), with this many threads.
    Video { threads: usize },
    /// Loose review PNGs at these times.
    Frames(Vec<f64>),
    /// The contact sheet at these times, or at [`Reel::key_times`] if `None`.
    Sheet(Option<Vec<f64>>),
    /// Only `events.json`.
    Events,
}

impl Command {
    /// Reads `--frames 1.2,4.0`, `--sheet [1,3,5]`, `--serial` (one thread),
    /// `--events`, or nothing (the MP4 on every core).
    pub fn from_args(args: &[String]) -> Result<Command, ReelError> {
        let value = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .map(|i| args.get(i + 1).cloned())
        };
        if let Some(list) = value("--frames") {
            let list = list
                .ok_or_else(|| ReelError::Usage("--frames needs times: --frames 1.2,4.0".into()))?;
            return Ok(Command::Frames(times(&list)?));
        }
        if let Some(list) = value("--sheet") {
            return Ok(Command::Sheet(
                match list.filter(|s| !s.starts_with("--")) {
                    Some(s) => Some(times(&s)?),
                    None => None,
                },
            ));
        }
        if args.iter().any(|a| a == "--events") {
            return Ok(Command::Events);
        }
        let threads = if args.iter().any(|a| a == "--serial") {
            1
        } else {
            all_cores()
        };
        Ok(Command::Video { threads })
    }
}

/// `1.2,4.0` → `[1.2, 4.0]`, naming the item that is not a number.
pub fn times(list: &str) -> Result<Vec<f64>, ReelError> {
    list.split(',')
        .map(|x| {
            x.trim()
                .parse()
                .map_err(|_| ReelError::Usage(format!("«{x}» is not a time in seconds")))
        })
        .collect()
}

/// One thread per core (1 if the platform cannot say).
pub fn all_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Loads the faces the sink draws text with; called once per sink.
pub type FontLoader = Box<dyn Fn() -> std::io::Result<Fonts> + Sync>;

/// A reel: its name (the MP4's), folder, length, background, clock and canvas.
pub struct Reel {
    pub name: String,
    pub dir: PathBuf,
    pub duration: f64,
    pub background: Rgb,
    pub clock: Clock,
    pub canvas: Canvas,
    pub layout: Layout,
    pub grid: SheetGrid,
    /// The face the contact sheet labels each cell with; `None` leaves the
    /// cells unlabelled (a reel without fonts).
    pub label_face: Option<usize>,
    fonts: FontLoader,
}

impl Reel {
    /// A reel on `canvas` whose sinks load their faces with `fonts`.
    pub fn new(
        name: &str,
        dir: PathBuf,
        duration: f64,
        background: Rgb,
        clock: Clock,
        canvas: Canvas,
        fonts: FontLoader,
    ) -> Reel {
        Reel {
            name: name.to_string(),
            dir,
            duration,
            background,
            clock,
            canvas,
            layout: Layout::default(),
            grid: SheetGrid::default(),
            label_face: None,
            fonts,
        }
    }

    pub fn with_layout(mut self, layout: Layout) -> Reel {
        self.layout = layout;
        self
    }

    pub fn with_label_face(mut self, face: usize) -> Reel {
        self.label_face = Some(face);
        self
    }

    pub fn with_grid(mut self, grid: SheetGrid) -> Reel {
        self.grid = grid;
        self
    }

    /// Frames in the MP4.
    pub fn frame_count(&self) -> usize {
        self.canvas.frames(self.duration)
    }

    /// The MP4's path: `<dir>/<name>.mp4`.
    pub fn mp4(&self) -> PathBuf {
        self.dir.join(format!("{}.mp4", self.name))
    }

    /// Does what `command` asks; returns the files it wrote, in order.
    pub fn run<L, F>(&self, command: &Command, frame: F) -> Result<Vec<PathBuf>, ReelError>
    where
        L: Overlay,
        F: Fn(f64) -> Frame<L> + Sync,
    {
        match command {
            Command::Frames(ts) => self.frames_png(ts, &frame),
            Command::Sheet(ts) => {
                let ts = ts.clone().unwrap_or_else(|| self.key_times());
                Ok(vec![self.contact_sheet(&ts, &frame)?])
            }
            Command::Events => Ok(vec![self.write_events()?]),
            Command::Video { threads } => {
                let events = self.write_events()?;
                let mp4 = self.mp4();
                self.video(&mp4, *threads, &frame)?;
                Ok(vec![events, mp4])
            }
        }
    }

    /// The default key frames: the middle of each stretch between two events
    /// (and between the last event and the end), rounded to a frame. At the
    /// start of a beat the frame is often still empty; by its middle what
    /// came in has settled.
    pub fn key_times(&self) -> Vec<f64> {
        let fps = self.canvas.fps;
        let mut ts: Vec<f64> = self.clock.events().iter().map(|(_, t)| *t).collect();
        ts.push(self.duration);
        ts.sort_by(f64::total_cmp);
        ts.dedup();
        ts.windows(2)
            .map(|w| ((w[0] + w[1]) / 2.0 * fps).round() / fps)
            .collect()
    }

    /// Writes the clock next to the reel (AC1).
    pub fn write_events(&self) -> Result<PathBuf, ReelError> {
        let path = self.dir.join(&self.layout.events);
        std::fs::write(&path, self.clock.to_json(self.duration, self.canvas.fps))?;
        Ok(path)
    }

    fn sink(&self, dir: &Path) -> Result<PpmSink, ReelError> {
        let c = self.canvas;
        Ok(PpmSink::with_view(dir, (c.width, c.height), c.view())?
            .with_background(self.background)
            .with_fonts((self.fonts)()?))
    }

    /// Rasterizes a frame and composes its overlays: the whole P6.
    fn rasterize<L: Overlay>(
        sink: &mut PpmSink,
        dir: &Path,
        i: usize,
        (prims, overlays): Frame<L>,
    ) -> Result<Vec<u8>, ReelError> {
        sink.frame(i, &prims)?;
        let file = dir.join(format!("frame_{i:05}.ppm"));
        let mut bytes = std::fs::read(&file)?;
        std::fs::remove_file(&file)?;
        for o in &overlays {
            o.compose(&mut bytes)?;
        }
        Ok(bytes)
    }

    /// The RGB pixels of one of motoreel's P6 frames (the header has fixed length).
    fn pixels<'b>(&self, p6: &'b [u8]) -> &'b [u8] {
        &p6[p6.len() - self.canvas.rgb_len()..]
    }

    /// The MP4 with `threads` threads, with the delivery flags of
    /// guion-encode (H.264, BT.709 tagged, faststart). The same bytes for any
    /// number of threads (AC2).
    pub fn video<L, F>(&self, mp4: &Path, threads: usize, frame: &F) -> Result<(), ReelError>
    where
        L: Overlay,
        F: Fn(f64) -> Frame<L> + Sync,
    {
        let tmp = mp4.with_extension("frames");
        let spec = self.canvas.video_spec();
        let fps = self.canvas.fps;
        let done = encode_ordered_with(
            mp4,
            spec,
            self.frame_count(),
            threads,
            |h| -> Result<(PathBuf, PpmSink), ReelError> {
                let dir = tmp.join(format!("h{h}"));
                let sink = self.sink(&dir)?;
                Ok((dir, sink))
            },
            |(dir, sink), i| {
                // t comes from the index, never from a running sum
                let mut bytes = Reel::rasterize(sink, dir, i, frame(i as f64 / fps))?;
                bytes.drain(..bytes.len() - spec.frame_len());
                Ok::<_, ReelError>(bytes)
            },
        );
        let _ = std::fs::remove_dir_all(&tmp);
        done.map(|_| ()).map_err(ReelError::from)
    }

    /// Loose review frames: `<review>/t-<t>.png`, one per time.
    pub fn frames_png<L, F>(&self, times: &[f64], frame: &F) -> Result<Vec<PathBuf>, ReelError>
    where
        L: Overlay,
        F: Fn(f64) -> Frame<L>,
    {
        let dir = self.dir.join(&self.layout.review);
        let tmp = dir.join(".ppm");
        std::fs::create_dir_all(&dir)?;
        let mut sink = self.sink(&tmp)?;
        let mut done = Vec::new();
        for (i, &t) in times.iter().enumerate() {
            let bytes = Reel::rasterize(&mut sink, &tmp, i, frame(t))?;
            let png = dir.join(format!("t-{t:05.2}.png"));
            png_from_rgb(
                self.pixels(&bytes),
                self.canvas.width as usize,
                self.canvas.height as usize,
                &png,
            )?;
            done.push(png);
        }
        let _ = std::fs::remove_dir(&tmp);
        Ok(done)
    }

    /// The contact sheet: the frames at `times`, reduced, in rows of
    /// [`SheetGrid::columns`], each labelled with its time and the event it
    /// follows. Goes to `<review>/<sheet>` (AC3).
    pub fn contact_sheet<L, F>(&self, times: &[f64], frame: &F) -> Result<PathBuf, ReelError>
    where
        L: Overlay,
        F: Fn(f64) -> Frame<L>,
    {
        let SheetGrid {
            columns,
            reduction: k,
            gap,
        } = self.grid;
        let (fw, fh) = (self.canvas.width as usize, self.canvas.height as usize);
        let (w, h) = (fw / k, fh / k);
        let rows = times.len().div_ceil(columns);
        let (aw, ah) = (
            columns * w + (columns + 1) * gap,
            rows * h + (rows + 1) * gap,
        );
        let mut sheet = vec![0u8; aw * ah * 3];
        let dir = self.dir.join(&self.layout.review);
        let tmp = dir.join(".sheet");
        std::fs::create_dir_all(&dir)?;
        let mut sink = self.sink(&tmp)?;
        for (n, &t) in times.iter().enumerate() {
            let (mut prims, overlays) = frame(t);
            prims.extend(self.label(t));
            let bytes = Reel::rasterize(&mut sink, &tmp, n, (prims, overlays))?;
            let src = self.pixels(&bytes);
            let (x0, y0) = (
                gap + (n % columns) * (w + gap),
                gap + (n / columns) * (h + gap),
            );
            // the mean of each k×k block: no aliasing on thin edges
            for y in 0..h {
                for x in 0..w {
                    let mut acc = [0u32; 3];
                    for dy in 0..k {
                        for dx in 0..k {
                            let i = 3 * ((y * k + dy) * fw + x * k + dx);
                            for c in 0..3 {
                                acc[c] += src[i + c] as u32;
                            }
                        }
                    }
                    let d = 3 * ((y0 + y) * aw + x0 + x);
                    for c in 0..3 {
                        sheet[d + c] = ((acc[c] + (k * k / 2) as u32) / (k * k) as u32) as u8;
                    }
                }
            }
        }
        let _ = std::fs::remove_dir(&tmp);
        let png = dir.join(&self.layout.sheet);
        png_from_rgb(&sheet, aw, ah, &png)?;
        Ok(png)
    }

    /// The label text of a contact-sheet cell: its time and the last event at
    /// or before it, with how long after ("20.65 s · pasa +2.55").
    pub fn label_text(&self, t: f64) -> String {
        let fps = self.canvas.fps;
        let previous = self
            .clock
            .events()
            .iter()
            .filter(|(_, te)| *te <= t + 1e-9)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        match previous {
            Some((n, te)) if (t - te).abs() < 0.5 / fps => format!("{t:.2} s · {n}"),
            Some((n, te)) => format!("{t:.2} s · {n} +{:.2}", t - te),
            None => format!("{t:.2} s"),
        }
    }

    /// A contact-sheet cell's label: a dark band across the top and the text
    /// in white, sized for a 1080 px wide canvas and scaled to this one.
    /// Empty without a [`Reel::label_face`].
    pub fn label(&self, t: f64) -> Vec<Prim2> {
        let Some(face) = self.label_face else {
            return Vec::new();
        };
        let c = self.canvas;
        let k = c.width as f64 / 1080.0;
        let white = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let black = Rgb { r: 0, g: 0, b: 0 };
        let band = [
            (0.0, 0.0),
            (1080.0 * k, 0.0),
            (1080.0 * k, 120.0 * k),
            (0.0, 120.0 * k),
        ]
        .map(|(x, y)| c.to_view(x, y));
        vec![
            Prim2::Path {
                subpaths: polygon(&band),
                // no stroke, so the edge does not double the band's opacity
                style: Style {
                    stroke: black,
                    width: 0.0,
                    alpha: 0.0,
                    fill: Some(Fill::solid(black, 0.78)),
                },
            },
            Prim2::Text {
                at: c.to_view(36.0 * k, 92.0 * k),
                text: self.label_text(t),
                size: c.len(76.0 * k),
                align: Align::Left,
                face,
                style: Style {
                    stroke: white,
                    width: 0.0,
                    alpha: 1.0,
                    fill: None,
                },
            },
        ]
    }
}

/// A PNG from an RGB24 buffer, through ffmpeg (the one guion-encode finds).
pub fn png_from_rgb(rgb: &[u8], width: usize, height: usize, png: &Path) -> Result<(), ReelError> {
    let mut ff = Process::new(guion_encode::ffmpeg::ffmpeg()?)
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
        ])
        .args(["-s", &format!("{width}x{height}"), "-i", "-"])
        .arg(png)
        .stdin(Stdio::piped())
        .spawn()?;
    ff.stdin
        .take()
        .expect("ffmpeg's stdin was piped")
        .write_all(rgb)?;
    if !ff.wait()?.success() {
        return Err(ReelError::Io(std::io::Error::other(format!(
            "ffmpeg could not write {}",
            png.display()
        ))));
    }
    Ok(())
}
