//! One post open in the studio (R-0015 AC6): its scene in memory, its
//! voice, its account's template, and what the page asks of it.
//!
//! Everything the page shows is computed here from the scene as it is in
//! memory; nothing is written until [`Estudio::guardar`], which writes
//! `escena.toml` (the source of truth) in its canonical form.

use std::path::{Path, PathBuf};

use guion_gadget::escena::ARCHIVO;
use guion_gadget::{
    Compilada, Contexto, Dato, Escena, Origen, Procedencia, Registro, Tiempos, Tipo,
};
use guion_reel::{Command, Frame, Narration, NoOverlay, Reel};
use motoreel_typeset::Fonts;
use serde_json::{json, Value};

use crate::plantilla::Plantilla;

/// A post open in the studio.
pub struct Estudio {
    pub post: PathBuf,
    pub escena: Escena,
    pub registro: Registro,
    pub plantilla: Plantilla,
    pub voz: Option<Narration>,
    caras: Option<Fonts>,
}

impl Estudio {
    /// Opens `post`: its `escena.toml` (an empty scene if there is none
    /// yet), its `voz.json` if synthesised, and its account's template.
    pub fn abrir(post: &Path, registro: Registro, cara: Option<&Path>) -> Result<Estudio, String> {
        if !post.is_dir() {
            return Err(format!("{}: not a post folder", post.display()));
        }
        let escena = if post.join(ARCHIVO).is_file() {
            Escena::load(post)?
        } else {
            Escena::vacia()
        };
        let vj = post.join("voz.json");
        let voz = if vj.is_file() {
            Some(Narration::load(&vj).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let plantilla = Plantilla::para(post, cara)?;
        // a gadget of the account is served by the account's own binary
        if let Some(e) = plantilla.cuenta.as_ref().and_then(|c| c.estudio.as_ref()) {
            if let Some(g) = escena
                .gadgets
                .iter()
                .find(|g| registro.buscar(&g.tipo).is_none() && e.gadgets.contains(&g.tipo))
            {
                let raiz = plantilla.raiz.clone().unwrap_or_default();
                return Err(format!(
                    "gadget «{}» is «{}», a gadget of this account: serve the studio with its binary, \
                     cargo run --manifest-path {} --bin {} -- {}",
                    g.id,
                    g.tipo,
                    raiz.join(&e.motor).display(),
                    e.bin,
                    post.display()
                ));
            }
        }
        let caras = plantilla.fuentes()?;
        let est = Estudio {
            post: post.to_path_buf(),
            escena,
            registro,
            plantilla,
            voz,
            caras,
        };
        // fail at open, naming the first problem, rather than on every frame
        est.compilada_con(&est.contexto())?;
        Ok(est)
    }

    /// The context every gadget evaluates in.
    pub fn contexto(&self) -> Contexto<'_> {
        let p = &self.plantilla;
        Contexto {
            canvas: p.canvas,
            paleta: &p.paleta,
            caras: self.caras.as_ref(),
            cara: p.cara,
            seguro: p.seguro,
        }
    }

    /// The reel's length.
    pub fn duracion(&self) -> Result<f64, String> {
        self.escena
            .duracion_con(self.voz.as_ref(), self.plantilla.canvas.fps)
    }

    /// The instants anchors resolve against.
    pub fn tiempos(&self) -> Result<Tiempos, String> {
        Ok(Tiempos::de_voz(self.voz.as_ref(), self.duracion()?))
    }

    fn compilada_con<'a>(&'a self, cx: &Contexto) -> Result<Compilada<'a>, String> {
        self.escena.compilar(&self.registro, &self.tiempos()?, cx)
    }

    /// The studio of guion-reel for this post: its canvas, background and
    /// faces, `eventos.json`, review frames in `cuadros/`.
    pub fn reel(&self) -> Result<Reel, String> {
        let p = &self.plantilla;
        let nombre = self
            .post
            .canonicalize()
            .ok()
            .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "reel".into());
        let cx = self.contexto();
        let reloj = self
            .compilada_con(&cx)?
            .reloj(self.voz.as_ref(), p.canvas.fps, &cx)?;
        Ok(Reel::new(
            &nombre,
            self.post.clone(),
            self.duracion()?,
            p.fondo(),
            reloj,
            p.canvas,
            p.cargador(),
        )
        .with_layout(guion_reel::Layout {
            events: "eventos.json".into(),
            review: "cuadros".into(),
            sheet: "hoja.png".into(),
        }))
    }

    /// The frame at `t` as RGB24, rendered by guion-reel (AC6).
    pub fn cuadro_rgb(&self, t: f64) -> Result<Vec<u8>, String> {
        let cx = self.contexto();
        let c = self.compilada_con(&cx)?;
        let reel = self.reel_ligero()?;
        let frame = |t: f64| -> Frame<NoOverlay> { (c.cuadro(t, &cx).prims, Vec::new()) };
        reel.frame_rgb(t, &frame).map_err(|e| e.to_string())
    }

    /// A reel without its clock (a frame does not need `eventos.json`).
    fn reel_ligero(&self) -> Result<Reel, String> {
        let p = &self.plantilla;
        Ok(Reel::new(
            "cuadro",
            self.post.clone(),
            self.duracion()?,
            p.fondo(),
            guion_reel::Clock::new(),
            p.canvas,
            p.cargador(),
        ))
    }

    /// The frame at `t` as a PNG, reduced `reduccion` times (the mean of
    /// each block, as the contact sheet does). The same scene and `t`, the
    /// same bytes.
    pub fn cuadro_png(&self, t: f64, reduccion: usize) -> Result<Vec<u8>, String> {
        let rgb = self.cuadro_rgb(t)?;
        let (w, h) = (
            self.plantilla.canvas.width as usize,
            self.plantilla.canvas.height as usize,
        );
        let (rgb, w, h) = reducir(&rgb, w, h, reduccion.max(1));
        png(&rgb, w, h)
    }

    /// Everything the page shows, at `t`, as JSON: the template, the
    /// library, the anchors, each gadget's params (spec, value, binding,
    /// provenance, animated or not), values and avisos.
    pub fn estado(&self, t: f64) -> Result<Value, String> {
        let cx = self.contexto();
        let c = self.compilada_con(&cx)?;
        let q = c.cuadro(t, &cx);
        let p = &self.plantilla;
        let biblioteca: Vec<Value> = guion_gadget::registro::estantes(&self.registro)
            .into_iter()
            .map(|(cat, gs)| {
                json!({
                    "categoria": cat.nombre(),
                    "gadgets": gs.iter().map(|g| json!({"tipo": g.tipo(), "doc": g.doc()})).collect::<Vec<_>>(),
                })
            })
            .collect();
        let mut gadgets = Vec::new();
        for (ins, e) in self.escena.gadgets.iter().zip(&q.gadgets) {
            let g = self
                .registro
                .buscar(&ins.tipo)
                .ok_or_else(|| format!("no gadget «{}»", ins.tipo))?;
            let params: Vec<Value> = g
                .params()
                .iter()
                .map(|s| {
                    let escrita = ins.params.get(s.nombre);
                    let (min, max, paso, opciones) = match s.tipo {
                        Tipo::Num { min, max, paso, .. } => {
                            (json!(min), json!(max), json!(paso), json!(null))
                        }
                        Tipo::Opcion { opciones, .. } => {
                            (json!(null), json!(null), json!(null), json!(opciones))
                        }
                        _ => (json!(null), json!(null), json!(null), json!(null)),
                    };
                    let tipo = match s.tipo {
                        Tipo::Num { .. } => "num",
                        Tipo::Texto { .. } => "texto",
                        Tipo::Opcion { .. } => "opcion",
                        Tipo::Token { .. } => "token",
                    };
                    let de = match escrita.map(|x| &x.origen) {
                        Some(Origen::De(r)) => json!(r),
                        _ => json!(null),
                    };
                    json!({
                        "nombre": s.nombre,
                        "doc": s.doc,
                        "tipo": tipo,
                        "unidad": s.unidad(),
                        "min": min, "max": max, "paso": paso, "opciones": opciones,
                        "fisico": s.fisico,
                        "valor": dato_json(e.params.dato(s.nombre)),
                        "procedencia": e.params.procedencia(s.nombre).map(Procedencia::nombre),
                        "fuente": escrita.map(|x| x.fuente.as_str()).unwrap_or(""),
                        "nota": escrita.map(|x| x.nota.as_str()).unwrap_or(""),
                        "de": de,
                        "animado": ins.anims.iter().any(|a| a.param == s.nombre),
                        "escrito": escrita.is_some(),
                    })
                })
                .collect();
            gadgets.push(json!({
                "id": ins.id,
                "tipo": ins.tipo,
                "categoria": g.categoria().nombre(),
                "entra": ins.entra,
                "sale": ins.sale,
                "alfa": e.alfa,
                "params": params,
                "valores": e.valores.iter().map(|v| json!({
                    "nombre": v.nombre, "cantidad": v.cantidad, "unidad": v.unidad,
                    "procedencia": v.procedencia.nombre(), "nota": v.nota,
                })).collect::<Vec<_>>(),
                "eventos": e.eventos,
                "avisos": e.avisos,
            }));
        }
        let tiempos = self.tiempos()?;
        Ok(json!({
            "post": self.post.display().to_string(),
            "t": t,
            "duracion": c.duracion,
            "fps": p.canvas.fps,
            "lienzo": [p.canvas.width, p.canvas.height],
            "plantilla": {
                "nombre": p.nombre,
                "notas": p.notas,
                "colores": p.paleta.roles.keys().map(|r| json!({
                    "rol": r,
                    "hex": p.paleta.color(r).map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
                })).collect::<Vec<_>>(),
            },
            "biblioteca": biblioteca,
            "anclas": tiempos.nombres().iter().map(|(n, t)| json!({"nombre": n, "t": t})).collect::<Vec<_>>(),
            "voz": self.voz.as_ref().map(|v| v.lines.iter().map(|l| json!({
                "id": l.id, "t0": l.t0, "t1": l.t1, "subtitulo": l.subtitle,
            })).collect::<Vec<_>>()),
            "gadgets": gadgets,
        }))
    }

    /// Sets one param in memory (AC6), checked by the gadget's spec and then
    /// by the whole scene: a value that would break it is refused and the
    /// scene stays as it was.
    pub fn fijar(
        &mut self,
        id: &str,
        param: &str,
        d: Dato,
        procedencia: Option<Procedencia>,
    ) -> Result<(), String> {
        let antes = self.escena.clone();
        self.escena
            .fijar(&self.registro, id, param, d, procedencia)?;
        let cx = self.contexto();
        if let Err(e) = self.compilada_con(&cx) {
            self.escena = antes;
            return Err(e);
        }
        Ok(())
    }

    /// Writes `escena.toml` (AC6): what the studio edited is now the source.
    pub fn guardar(&self) -> Result<PathBuf, String> {
        let p = self.post.join(ARCHIVO);
        std::fs::write(&p, self.escena.a_toml(&self.registro))
            .map_err(|e| format!("{}: {e}", p.display()))?;
        Ok(p)
    }

    /// Every aviso of the reel, frame by frame (`guion studio --check`).
    pub fn comprobar(&self) -> Result<Vec<String>, String> {
        let cx = self.contexto();
        Ok(self
            .compilada_con(&cx)?
            .avisos(self.plantilla.canvas.fps, &cx))
    }

    /// Renders what `command` asks (the MP4, frames, the contact sheet or
    /// `eventos.json`), with guion-reel, from the scene as saved.
    pub fn render(&self, command: &Command) -> Result<Vec<PathBuf>, String> {
        let cx = self.contexto();
        let c = self.compilada_con(&cx)?;
        let reel = self.reel()?;
        let frame = |t: f64| -> Frame<NoOverlay> { (c.cuadro(t, &cx).prims, Vec::new()) };
        match command {
            // a frame per thread needs a shareable context; one thread is
            // byte-identical (R-0009 AC2), so the studio's own render is serial
            Command::Video { .. } => reel.run(&Command::Video { threads: 1 }, frame),
            other => reel.run(other, frame),
        }
        .map_err(|e| e.to_string())
    }
}

fn dato_json(d: Option<&Dato>) -> Value {
    match d {
        Some(Dato::Num(x)) => json!(x),
        Some(Dato::Texto(s)) => json!(s),
        None => Value::Null,
    }
}

/// Reduces an RGB24 image `k` times by averaging each k×k block.
pub fn reducir(rgb: &[u8], w: usize, h: usize, k: usize) -> (Vec<u8>, usize, usize) {
    if k <= 1 {
        return (rgb.to_vec(), w, h);
    }
    let (rw, rh) = (w / k, h / k);
    let mut out = vec![0u8; rw * rh * 3];
    let n = (k * k) as u32;
    for y in 0..rh {
        for x in 0..rw {
            let mut acc = [0u32; 3];
            for dy in 0..k {
                let fila = (y * k + dy) * w;
                for dx in 0..k {
                    let i = 3 * (fila + x * k + dx);
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += u32::from(rgb[i + c]);
                    }
                }
            }
            let d = 3 * (y * rw + x);
            for (c, a) in acc.iter().enumerate() {
                out[d + c] = ((a + n / 2) / n) as u8;
            }
        }
    }
    (out, rw, rh)
}

/// An RGB24 image as PNG (deterministic: fixed compression and filter).
pub fn png(rgb: &[u8], w: usize, h: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w as u32, h as u32);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        e.set_compression(png::Compression::Fast);
        let mut wr = e.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(rgb).map_err(|e| e.to_string())?;
        wr.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
}
