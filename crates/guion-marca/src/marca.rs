//! [`Marca`]: the neutral primitives of a reel, read from `cuenta.toml`
//! (R-0014 AC6).
//!
//! Sizes are px of em, as a design states them and as the gate measures the
//! subtitle (R-0012: `size_unit = "em"`): motoreel takes the face's line
//! height, so an em size is multiplied by `(ascent − descent) / unitsPerEm`.
//! The subtitle check is the gate's own ([`guion_reel::gate::subtitles`]),
//! against the profile derived from the same contract, so a scene that passes
//! its own assert passes the gate's subtitle checks.

use std::path::{Path, PathBuf};

use guion_cuenta::{perfil, Cuenta};
use guion_reel::ease::{ease_out_cubic, progress};
use guion_reel::gate::subtitles::{self, SubtitleMeter};
use guion_reel::gate::Profile;
use guion_reel::{Canvas, Clock, FontLoader, Layout, Narration, Reel};
use motoreel::shapes::polygon;
use motoreel::{Align, Fill, Prim2, Rgb, Style};
use motoreel_typeset::{measure, Face, FaceId, Fonts};

/// The account's two faces, in the order the sinks load them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cara {
    Titulo = 0,
    Texto = 1,
}

/// An account's brand, loaded.
pub struct Marca {
    pub cuenta: Cuenta,
    /// The account's root (where `cuenta.toml` is).
    pub raiz: PathBuf,
    pub canvas: Canvas,
    perfil: Profile,
    bytes: [Vec<u8>; 2],
    linea_en_em: [f64; 2],
    medir: Fonts,
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb {
        r: c[0],
        g: c[1],
        b: c[2],
    }
}

fn linea_en_em(bytes: &[u8], nombre: &str) -> Result<f64, String> {
    use ab_glyph::Font as _;
    let f =
        ab_glyph::FontRef::try_from_slice(bytes).map_err(|_| format!("{nombre}: not a font"))?;
    let upem = f
        .units_per_em()
        .ok_or_else(|| format!("{nombre}: no unitsPerEm"))?;
    Ok(f64::from(f.ascent_unscaled() - f.descent_unscaled()) / f64::from(upem))
}

fn cargar_fuentes(bytes: &[Vec<u8>; 2]) -> std::io::Result<Fonts> {
    let mut f = Fonts::new();
    for (i, b) in bytes.iter().enumerate() {
        let cara = Face::load(b.clone(), ["titulo", "texto"][i])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        f.add(cara);
    }
    Ok(f)
}

/// Multiplies the opacity of every primitive (stroke and fill) by `a`.
pub fn con_alfa(mut v: Vec<Prim2>, a: f64) -> Vec<Prim2> {
    for p in &mut v {
        let s = match p {
            Prim2::Point { style, .. }
            | Prim2::Segment { style, .. }
            | Prim2::Polyline { style, .. }
            | Prim2::Edges { style, .. }
            | Prim2::Text { style, .. }
            | Prim2::Path { style, .. } => style,
        };
        s.alpha *= a;
        if let Some(f) = &mut s.fill {
            f.alpha *= a;
        }
    }
    v
}

impl Marca {
    /// Reads `<raiz>/cuenta.toml` and the faces it names (relative to the
    /// root, or absolute).
    pub fn cargar(raiz: &Path) -> Result<Marca, String> {
        let cuenta = Cuenta::load(raiz)?;
        let mut p = Profile::parse(&perfil::perfil(&cuenta)).map_err(|e| e.to_string())?;
        p.dir = raiz.join("brand");
        let leer = |rel: &str| -> Result<Vec<u8>, String> {
            let f = raiz.join(rel);
            std::fs::read(&f).map_err(|e| {
                format!(
                    "{}: {e} (the faces are the account's: brand/fonts/README.md)",
                    f.display()
                )
            })
        };
        let f = &cuenta.marca.fuentes;
        let bytes = [leer(&f.titulo)?, leer(&f.texto)?];
        let linea = [
            linea_en_em(&bytes[0], &f.titulo)?,
            linea_en_em(&bytes[1], &f.texto)?,
        ];
        let medir = cargar_fuentes(&bytes).map_err(|e| e.to_string())?;
        let l = cuenta.marca.lienzo;
        Ok(Marca {
            canvas: Canvas {
                width: l.ancho,
                height: l.alto,
                fps: l.fps,
                px_per_unit: f64::from(l.ancho) / 1.8,
            },
            cuenta,
            raiz: raiz.to_path_buf(),
            perfil: p,
            bytes,
            linea_en_em: linea,
            medir,
        })
    }

    /// The gate profile derived from the same contract.
    pub fn perfil(&self) -> &Profile {
        &self.perfil
    }

    /// What every sink loads its faces with.
    pub fn fuentes(&self) -> FontLoader {
        let bytes = self.bytes.clone();
        Box::new(move || cargar_fuentes(&bytes))
    }

    /// A token's colour. An unknown name is a bug in the scene: it panics
    /// naming the token and the ones there are.
    pub fn color(&self, token: &str) -> Rgb {
        match self.cuenta.marca.tokens.get(token) {
            Some(c) => rgb(*c),
            None => panic!(
                "token «{token}» is not in [marca.tokens] (there are: {})",
                self.cuenta
                    .marca
                    .tokens
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// The background a frame is cleared to (the first `fondo` role).
    pub fn fondo(&self) -> Rgb {
        self.color(&self.cuenta.marca.roles.fondo[0])
    }

    /// The text colour.
    pub fn tinta(&self) -> Rgb {
        self.color(&self.cuenta.marca.roles.texto)
    }

    /// The accent (the first `acento` role).
    pub fn acento(&self) -> Rgb {
        self.color(&self.cuenta.marca.roles.acento[0])
    }

    /// The size motoreel takes for `em` px of em in `cara`.
    pub fn tam(&self, cara: Cara, em: f64) -> f64 {
        em * self.linea_en_em[cara as usize]
    }

    /// The advance of `s` set in `cara` at `em` px, in px.
    pub fn ancho(&self, s: &str, cara: Cara, em: f64) -> f64 {
        measure(s, FaceId(cara as usize), self.tam(cara, em), &self.medir)
            .unwrap_or_else(|e| panic!("«{s}»: {e}"))
            .metrics()
            .width
    }

    /// One line of text with its baseline at `y` px, aligned on `x`.
    #[allow(clippy::too_many_arguments)]
    pub fn texto(
        &self,
        s: &str,
        cara: Cara,
        em: f64,
        color: Rgb,
        x: f64,
        y: f64,
        align: Align,
        alfa: f64,
    ) -> Vec<Prim2> {
        if s.is_empty() || alfa <= 0.0 {
            return Vec::new();
        }
        let w = self.ancho(s, cara, em);
        let x0 = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        vec![Prim2::Text {
            at: self.canvas.to_view(x0, y),
            text: s.to_string(),
            size: self.canvas.len(self.tam(cara, em)),
            align: Align::Left,
            face: cara as usize,
            style: Style {
                stroke: color,
                width: 0.0,
                alpha: alfa,
                fill: None,
            },
        }]
    }

    /// The widest a line may be: the safe area less a margin for the edges.
    fn ancho_util(&self) -> f64 {
        let x = self.cuenta.marca.seguro.x;
        x[1] - x[0] - 24.0
    }

    /// `em`, or less if `s` would not fit the safe area.
    fn ajustar(&self, s: &str, cara: Cara, em: f64) -> f64 {
        let w = self.ancho(s, cara, em);
        if w > self.ancho_util() {
            em * self.ancho_util() / w
        } else {
            em
        }
    }

    /// The title: the title face, centred, on `[marca.titulo]`'s baseline,
    /// shrunk to fit the safe area.
    pub fn titulo(&self, s: &str, alfa: f64) -> Vec<Prim2> {
        let t = self.cuenta.marca.titulo;
        let em = self.ajustar(s, Cara::Titulo, t.tam);
        let cx = f64::from(self.canvas.width) / 2.0;
        self.texto(
            s,
            Cara::Titulo,
            em,
            self.tinta(),
            cx,
            t.y,
            Align::Center,
            alfa,
        )
    }

    /// A filled rectangle centred on `cx`, `[y0, y0 + alto]`.
    pub fn barra(
        &self,
        cx: f64,
        y0: f64,
        ancho: f64,
        alto: f64,
        color: Rgb,
        alfa: f64,
    ) -> Vec<Prim2> {
        if ancho < 1.0 || alfa <= 0.0 {
            return Vec::new();
        }
        let (x0, x1, y1) = (cx - ancho / 2.0, cx + ancho / 2.0, y0 + alto);
        let pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| self.canvas.to_view(x, y));
        vec![Prim2::Path {
            subpaths: polygon(&pts),
            style: Style {
                stroke: color,
                width: 0.0,
                alpha: 0.0,
                fill: Some(Fill::solid(color, alfa)),
            },
        }]
    }

    /// The grid of `[marca.rejilla]` (nothing without one): one filled path
    /// of every line and pad, in its background token. Drawn first.
    pub fn rejilla(&self) -> Vec<Prim2> {
        let Some(g) = &self.cuenta.marca.rejilla else {
            return Vec::new();
        };
        let (w, h) = (f64::from(self.canvas.width), f64::from(self.canvas.height));
        let c = self.color(&g.token);
        let mut sub = Vec::new();
        let mut rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            let pts =
                [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| self.canvas.to_view(x, y));
            sub.extend(polygon(&pts));
        };
        let media = g.grosor / 2.0;
        // centred on the canvas, so the margins match on both sides
        let x0 = (w / 2.0) % g.paso;
        let y0 = (h / 2.0) % g.paso;
        let xs: Vec<f64> = (0..)
            .map(|i| x0 + i as f64 * g.paso)
            .take_while(|x| *x <= w)
            .collect();
        let ys: Vec<f64> = (0..)
            .map(|i| y0 + i as f64 * g.paso)
            .take_while(|y| *y <= h)
            .collect();
        for &x in &xs {
            rect(x - media, 0.0, x + media, h);
        }
        for &y in &ys {
            rect(0.0, y - media, w, y + media);
        }
        if let Some(n) = std::num::NonZeroUsize::new(g.nodos) {
            let lado = g.grosor * 2.5;
            for (i, &x) in xs.iter().enumerate().filter(|(i, _)| *i % n == 0) {
                for (j, &y) in ys.iter().enumerate().filter(|(j, _)| *j % n == 0) {
                    if (i / n + j / n) % 2 == 0 {
                        rect(x - lado, y - lado, x + lado, y + lado);
                    }
                }
            }
        }
        vec![Prim2::Path {
            subpaths: sub,
            style: Style {
                stroke: c,
                width: 0.0,
                alpha: 0.0,
                fill: Some(Fill::solid(c, 1.0)),
            },
        }]
    }

    /// The subtitle at `t`: the line being said, with its fade, in the text
    /// face, centred on the subtitle's baseline.
    pub fn subtitulo(&self, voz: &Narration, t: f64) -> Vec<Prim2> {
        let s = &self.cuenta.marca.subtitulo;
        let Some((l, a)) = voz.active(t, s.fundido) else {
            return Vec::new();
        };
        let cx = f64::from(self.canvas.width) / 2.0;
        self.texto(
            &l.subtitle,
            Cara::Texto,
            s.tam,
            self.tinta(),
            cx,
            s.base,
            Align::Center,
            a,
        )
    }

    /// The gate's subtitle checks (width, words, signs, long lines, overlap)
    /// on a narration; `Err` lists every failure.
    pub fn comprobar_subtitulos(&self, voz: &Narration) -> Result<(), String> {
        let meter = SubtitleMeter::new(&self.perfil).map_err(|e| e.to_string())?;
        let fallas: Vec<String> = subtitles::check(voz, &self.perfil, &meter)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|f| !f.pass)
            .map(|f| format!("{}: {} {}", f.check, f.detail, f.at.join("; ")))
            .collect();
        if fallas.is_empty() {
            Ok(())
        } else {
            Err(fallas.join("\n"))
        }
    }

    /// The closing card, `t` s after it starts: the account's name, its
    /// handle, and the accent bar growing under them; still from 0.6 s on.
    pub fn cierre(&self, t: f64) -> Vec<Prim2> {
        if t < 0.0 {
            return Vec::new();
        }
        let (w, h) = (f64::from(self.canvas.width), f64::from(self.canvas.height));
        let cx = w / 2.0;
        let y = (h * 0.47).round();
        let a = ease_out_cubic(progress(t, 0.0, 0.4));
        let c = &self.cuenta;
        let nombre = &c.cuenta.nombre;
        let em = self.ajustar(nombre, Cara::Titulo, 72.0);
        let mut v = self.texto(
            nombre,
            Cara::Titulo,
            em,
            self.tinta(),
            cx,
            y,
            Align::Center,
            a,
        );
        let handle = &c.cuenta.handle;
        if !handle.is_empty() {
            let sec = self.color(&c.marca.roles.secundario);
            let em = self.ajustar(handle, Cara::Texto, 44.0);
            v.extend(self.texto(handle, Cara::Texto, em, sec, cx, y + 80.0, Align::Center, a));
        }
        let largo = 240.0 * ease_out_cubic(progress(t, 0.1, 0.6));
        v.extend(self.barra(cx, y + 120.0, largo, 10.0, self.acento(), 1.0));
        v
    }

    /// The voice's clock: one `voz:<id>` event at the start of each line
    /// (the score and the scene read the same instants).
    pub fn reloj(&self, voz: &Narration) -> Result<Clock, String> {
        let mut c = Clock::new();
        for l in &voz.lines {
            c = c
                .event(&format!("voz:{}", l.id), l.t0)
                .map_err(|e| e.to_string())?;
        }
        Ok(c)
    }

    /// The studio for a reel of this account: its canvas, background and
    /// faces; `eventos.json`, and review frames in `cuadros/` (the contact
    /// sheet is `cuadros/hoja.png`, where the dashboard looks for it).
    pub fn reel(&self, nombre: &str, dir: PathBuf, duracion: f64, reloj: Clock) -> Reel {
        Reel::new(
            nombre,
            dir,
            duracion,
            self.fondo(),
            reloj,
            self.canvas,
            self.fuentes(),
        )
        .with_layout(Layout {
            events: "eventos.json".into(),
            review: "cuadros".into(),
            sheet: "hoja.png".into(),
        })
        .with_label_face(Cara::Texto as usize)
    }
}
