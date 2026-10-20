//! The account's template (R-0015 AC6): canvas, palette by role, faces and
//! safe area, read from the `cuenta.toml` above the post, so the same
//! gadgets take each account's look. A post outside any account gets the
//! neutral template.

use std::path::{Path, PathBuf};

use guion_cuenta::Cuenta;
use guion_gadget::Paleta;
use guion_reel::gate::Profile;
use guion_reel::{Canvas, FontLoader, Formato};
use motoreel::Rgb;
use motoreel_typeset::{Face, Fonts};

/// What the studio draws a post with.
pub struct Plantilla {
    /// The account's name, or «neutra».
    pub nombre: String,
    /// The account's root, if the post is in one.
    pub raiz: Option<PathBuf>,
    pub cuenta: Option<Cuenta>,
    pub canvas: Canvas,
    pub paleta: Paleta,
    /// The faces' bytes, in the order the sinks load them.
    pub caras: Vec<Vec<u8>>,
    /// The face labels are set in.
    pub cara: usize,
    pub seguro: [f64; 4],
    /// What is missing, said once (faces not dropped in yet).
    pub notas: Vec<String>,
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb {
        r: c[0],
        g: c[1],
        b: c[2],
    }
}

/// The folder holding the `cuenta.toml` nearest above `post` (itself included).
pub fn raiz_de_cuenta(post: &Path) -> Option<PathBuf> {
    let abs = post.canonicalize().ok()?;
    abs.ancestors()
        .find(|d| d.join(guion_cuenta::contrato::ARCHIVO).is_file())
        .map(Path::to_path_buf)
}

fn cargar_caras(bytes: &[Vec<u8>]) -> std::io::Result<Fonts> {
    let mut f = Fonts::new();
    for (i, b) in bytes.iter().enumerate() {
        let cara = Face::load(b.clone(), format!("cara{i}"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        f.add(cara);
    }
    Ok(f)
}

impl Plantilla {
    /// The neutral template: a vertical reel, the neutral palette, a 60 px
    /// safe margin, and the label face `cara` if one is given.
    pub fn neutra(cara: Option<&Path>) -> Result<Plantilla, String> {
        let c = Canvas::VERTICAL_REEL;
        let mut caras = Vec::new();
        let mut notas = Vec::new();
        match cara {
            Some(f) => caras.push(std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?),
            None => notas.push(
                "plantilla neutra sin caras: no se dibuja texto (--cara FUENTE.ttf)".to_string(),
            ),
        }
        Ok(Plantilla {
            nombre: "neutra".into(),
            raiz: None,
            cuenta: None,
            canvas: c,
            paleta: Paleta::neutra(),
            caras,
            cara: 0,
            seguro: [60.0, 60.0, c.width as f64 - 60.0, c.height as f64 - 60.0],
            notas,
        })
    }

    /// The template of the account at `raiz`: its contract's canvas, tokens
    /// and roles, safe area and faces (relative to the root, or absolute).
    /// Faces that are not there yet leave a note, not an error: the studio
    /// still edits and draws everything but text.
    pub fn de_cuenta(raiz: &Path) -> Result<Plantilla, String> {
        let cuenta = Cuenta::load(raiz)?;
        let m = &cuenta.marca;
        let tokens = m.tokens.iter().map(|(k, v)| (k.clone(), rgb(*v))).collect();
        let mut roles = std::collections::BTreeMap::new();
        let r = &m.roles;
        let mut rol = |n: &str, t: Option<&String>| {
            if let Some(t) = t {
                roles.insert(n.to_string(), t.clone());
            }
        };
        rol("fondo", r.fondo.first());
        rol("texto", Some(&r.texto));
        rol("secundario", Some(&r.secundario));
        rol("acento", r.acento.first());
        rol("foco", r.foco.first());
        let mut caras = Vec::new();
        let mut notas = Vec::new();
        for f in [&m.fuentes.titulo, &m.fuentes.texto] {
            match std::fs::read(raiz.join(f)) {
                Ok(b) => caras.push(b),
                Err(e) => notas.push(format!(
                    "{}: {e} (las caras son de la cuenta: brand/fonts/README.md); no se dibuja texto",
                    raiz.join(f).display()
                )),
            }
        }
        if caras.len() < 2 {
            caras.clear();
        }
        let l = m.lienzo;
        Ok(Plantilla {
            nombre: cuenta.cuenta.nombre.clone(),
            raiz: Some(raiz.to_path_buf()),
            canvas: Canvas {
                width: l.ancho,
                height: l.alto,
                fps: l.fps,
                px_per_unit: f64::from(l.ancho) / 1.8,
            },
            paleta: Paleta { tokens, roles },
            cara: 1,
            seguro: [m.seguro.x[0], m.seguro.y[0], m.seguro.x[1], m.seguro.y[1]],
            caras,
            notas,
            cuenta: Some(cuenta),
        })
    }

    /// The template for `post`: its account's, or the neutral one.
    pub fn para(post: &Path, cara: Option<&Path>) -> Result<Plantilla, String> {
        match raiz_de_cuenta(post) {
            Some(r) => Plantilla::de_cuenta(&r),
            None => Plantilla::neutra(cara),
        }
    }

    /// The template for format `f` (R-0016): its canvas and safe area. The
    /// account's own format (the orientation of its canvas) stays as it is,
    /// unless the account's gate profile (`brand/reel-perfil.toml`) writes a
    /// `[formato.<f>]`; another orientation takes the profile's bands for `f`
    /// (its overrides, else the house's defaults), at the account's fps, and
    /// 600 px per unit on the short side as before.
    pub fn para_formato(mut self, f: Formato) -> Result<Plantilla, String> {
        let perfil = match &self.raiz {
            Some(r) if r.join(guion_cuenta::perfil::RUTA).is_file() => {
                Some(Profile::load(&r.join(guion_cuenta::perfil::RUTA)).map_err(|e| e.to_string()))
            }
            _ => None,
        };
        let vertical = self.canvas.height > self.canvas.width;
        if f.vertical() == vertical {
            // the account's own orientation: only its profile's override moves it
            // (a profile that does not load is the gate's to report, not the studio's)
            let propio = matches!(&perfil, Some(Ok(p)) if p.formato.contains_key(f.nombre()));
            if !propio {
                return Ok(self);
            }
        }
        let perfil = perfil.transpose()?;
        let (w, h, fps, seguro) = match &perfil {
            Some(p) => {
                let q = p.para(f);
                let fps = if p
                    .formato
                    .get(f.nombre())
                    .is_some_and(|o| o.canvas.is_some())
                {
                    q.canvas.fps
                } else {
                    self.canvas.fps
                };
                (
                    q.canvas.width,
                    q.canvas.height,
                    fps,
                    [q.safe.x[0], q.safe.y[0], q.safe.x[1], q.safe.y[1]],
                )
            }
            None => {
                let b = f.bandas();
                (
                    b.width,
                    b.height,
                    self.canvas.fps,
                    [b.safe_x[0], b.safe_y[0], b.safe_x[1], b.safe_y[1]],
                )
            }
        };
        self.canvas = Canvas {
            width: w,
            height: h,
            fps,
            px_per_unit: f64::from(w.min(h)) / 1.8,
        };
        self.seguro = seguro;
        Ok(self)
    }

    /// The faces, loaded (None without any).
    pub fn fuentes(&self) -> Result<Option<Fonts>, String> {
        if self.caras.is_empty() {
            return Ok(None);
        }
        cargar_caras(&self.caras)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// What every sink loads its faces with.
    pub fn cargador(&self) -> FontLoader {
        let bytes = self.caras.clone();
        Box::new(move || cargar_caras(&bytes))
    }

    /// The background a frame is cleared to.
    pub fn fondo(&self) -> Rgb {
        self.paleta.color("fondo").unwrap_or(Rgb::BLACK)
    }
}
