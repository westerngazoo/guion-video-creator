//! `escena.toml`: the gadgets of a post, as the source of truth (R-0015 AC5).
//!
//! What the studio edits is this file, never a copy: a slider moves a value
//! in memory, «guardar» writes the file, and the render reads the file. The
//! voice stays in `voz.toml` (its times in `voz.json`) and the sound in
//! `sonido.toml`; this file only names their instants, through anchors.
//!
//! ```toml
//! version = 1
//! cola = 2.3                      # seconds after the voice (with voz.json)
//! # duracion = 8.0                # only for a post without voice
//!
//! [[gadget]]
//! id = "brazo"
//! tipo = "palanca"
//! entra = "gancho+0.5"            # anchors: inicio, fin, <line>, <line>.fin, ± s
//! sale = "fin"
//!
//! [gadget.params]
//! masa = { valor = 20.0, procedencia = "supuesto", nota = "la mancuerna que el reel declara" }
//! brazo = { valor = 0.32, procedencia = "reportado", fuente = "Winter, 0.20·H con H = 1.60 m" }
//! x = 540.0                       # layout: a bare value
//!
//! [[gadget.anim]]
//! param = "angulo"
//! de = 0.0
//! a = 90.0
//! desde = "gancho"
//! hasta = "gancho.fin"
//! curva = "suave"                 # lineal | suave | sale
//!
//! [[gadget]]
//! id = "tau"
//! tipo = "medidor"
//! [gadget.params]
//! valor = { de = "brazo.tau" }    # a binding: the output of an earlier gadget
//! ```
//!
//! **Determinism.** Parsing gives the same value or the same first error;
//! [`Escena::a_toml`] writes params in the gadget's declared order and
//! numbers in Rust's shortest round-trip form, so load → save → load is the
//! identity and saving twice gives the same bytes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use guion_reel::{Clock, Narration};
use motoreel::Prim2;
use serde::Deserialize;

use crate::dibujo::con_alfa;
use crate::gadget::{Contexto, Gadget, Salida};
use crate::linea::{animar, opacidad, Curva, Tiempos, Tramo};
use crate::registro::{tipo_valido, Registro};
use crate::valor::{Dato, Params, Procedencia, Tipo, Valor};

/// The file's name, in the post's folder.
pub const ARCHIVO: &str = "escena.toml";

/// The default tail after the voice (the closing card's 2.3 s).
pub const COLA: f64 = 2.3;

/// The default fade-in of a clip, s.
pub const FUNDIDO: f64 = 0.3;

/// Where a param's value comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Origen {
    /// Written in the file.
    Valor(Dato),
    /// Bound to an earlier gadget's output: `"<id>.<salida>"`.
    De(String),
}

/// One param as written: its value or binding, and its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    pub origen: Origen,
    pub procedencia: Option<Procedencia>,
    /// Where a measured, reported or computed value comes from.
    pub fuente: String,
    pub nota: String,
}

impl Entrada {
    /// A bare layout value.
    pub fn valor(d: Dato) -> Entrada {
        Entrada {
            origen: Origen::Valor(d),
            procedencia: None,
            fuente: String::new(),
            nota: String::new(),
        }
    }
}

/// An animation as written.
#[derive(Debug, Clone, PartialEq)]
pub struct Anim {
    pub param: String,
    pub de: f64,
    pub a: f64,
    pub desde: String,
    pub hasta: String,
    pub curva: Curva,
}

/// A gadget placed in the reel: a clip on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct Instancia {
    pub id: String,
    pub tipo: String,
    pub entra: String,
    pub sale: String,
    pub fundido: f64,
    pub params: BTreeMap<String, Entrada>,
    pub anims: Vec<Anim>,
}

/// The whole file.
#[derive(Debug, Clone, PartialEq)]
pub struct Escena {
    pub cola: f64,
    pub duracion: Option<f64>,
    pub gadgets: Vec<Instancia>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EscenaToml {
    version: u32,
    cola: Option<f64>,
    duracion: Option<f64>,
    #[serde(default)]
    gadget: Vec<GadgetToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GadgetToml {
    id: String,
    tipo: String,
    entra: Option<String>,
    sale: Option<String>,
    fundido: Option<f64>,
    #[serde(default)]
    params: toml::Table,
    #[serde(default)]
    anim: Vec<AnimToml>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnimToml {
    param: String,
    de: f64,
    a: f64,
    desde: String,
    hasta: String,
    curva: Option<String>,
}

fn dato(v: &toml::Value, donde: &str) -> Result<Dato, String> {
    match v {
        toml::Value::Float(x) => Ok(Dato::Num(*x)),
        toml::Value::Integer(i) => Ok(Dato::Num(*i as f64)),
        toml::Value::String(s) => Ok(Dato::Texto(s.clone())),
        otro => Err(format!(
            "{donde}: a number or a text, not a {}",
            otro.type_str()
        )),
    }
}

fn texto(v: &toml::Value, donde: &str) -> Result<String, String> {
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{donde}: a text"))
}

fn entrada(v: &toml::Value, donde: &str) -> Result<Entrada, String> {
    let toml::Value::Table(t) = v else {
        return Ok(Entrada::valor(dato(v, donde)?));
    };
    let mut origen = None;
    let mut e = Entrada::valor(Dato::Num(0.0));
    for (k, x) in t {
        let aqui = format!("{donde}.{k}");
        match k.as_str() {
            "valor" => origen = Some(Origen::Valor(dato(x, &aqui)?)),
            "de" => origen = Some(Origen::De(texto(x, &aqui)?)),
            "procedencia" => e.procedencia = Some(Procedencia::de(&texto(x, &aqui)?)?),
            "fuente" => e.fuente = texto(x, &aqui)?,
            "nota" => e.nota = texto(x, &aqui)?,
            _ => {
                return Err(format!(
                    "{donde}: unknown key «{k}» (valor, de, procedencia, fuente, nota)"
                ))
            }
        }
    }
    let tiene_valor = t.contains_key("valor");
    if tiene_valor && t.contains_key("de") {
        return Err(format!("{donde}: valor or de, not both"));
    }
    e.origen = origen.ok_or_else(|| format!("{donde}: needs valor or de"))?;
    Ok(e)
}

impl Escena {
    /// An empty scene (no gadgets) of a post with voice.
    pub fn vacia() -> Escena {
        Escena {
            cola: COLA,
            duracion: None,
            gadgets: Vec::new(),
        }
    }

    /// Parses the file's text: the grammar (unknown keys, types, the
    /// version). Meaning is checked by [`Escena::compilar`].
    pub fn parse(text: &str) -> Result<Escena, String> {
        let f: EscenaToml = toml::from_str(text).map_err(|e| e.to_string())?;
        if f.version != 1 {
            return Err(format!(
                "version = {}: this guion reads version 1",
                f.version
            ));
        }
        let mut gadgets = Vec::new();
        for g in f.gadget {
            let donde = format!("gadget «{}»", g.id);
            let mut params = BTreeMap::new();
            for (k, v) in &g.params {
                params.insert(k.clone(), entrada(v, &format!("{donde} param «{k}»"))?);
            }
            let mut anims = Vec::new();
            for a in g.anim {
                anims.push(Anim {
                    curva: match &a.curva {
                        Some(c) => Curva::de(c).map_err(|e| format!("{donde} anim: {e}"))?,
                        None => Curva::Suave,
                    },
                    param: a.param,
                    de: a.de,
                    a: a.a,
                    desde: a.desde,
                    hasta: a.hasta,
                });
            }
            gadgets.push(Instancia {
                id: g.id,
                tipo: g.tipo,
                entra: g.entra.unwrap_or_else(|| "inicio".into()),
                sale: g.sale.unwrap_or_else(|| "fin".into()),
                fundido: g.fundido.unwrap_or(FUNDIDO),
                params,
                anims,
            });
        }
        Ok(Escena {
            cola: f.cola.unwrap_or(COLA),
            duracion: f.duracion,
            gadgets,
        })
    }

    /// Reads `<post>/escena.toml`.
    pub fn load(post: &std::path::Path) -> Result<Escena, String> {
        let p = post.join(ARCHIVO);
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        Escena::parse(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// The reel's length: the voice's plus the tail, rounded to a frame;
    /// without voice, `duracion`. Both, or neither, is an error.
    pub fn duracion_con(&self, voz: Option<&Narration>, fps: f64) -> Result<f64, String> {
        match (voz, self.duracion) {
            (Some(_), Some(_)) => Err(
                "duracion: this post has voz.json, and the voice sets the length (use cola)".into(),
            ),
            (Some(v), None) => Ok(((v.duration + self.cola) * fps).round() / fps),
            (None, Some(d)) if d.is_finite() && d > 0.0 => Ok(d),
            (None, Some(d)) => Err(format!("duracion = {d}: a length > 0")),
            (None, None) => Err(
                "no voz.json and no duracion: synthesise the voice (guion reel voice) or, for a post without one, write duracion"
                    .into(),
            ),
        }
    }

    /// Checks the meaning against the registry, the instants and the
    /// palette, and resolves everything a frame needs (AC1, AC3–AC5). The
    /// first failure, in file order, naming the gadget and the param.
    pub fn compilar<'r>(
        &self,
        reg: &'r Registro,
        tiempos: &Tiempos,
        cx: &Contexto,
    ) -> Result<Compilada<'r>, String> {
        let mut items: Vec<Item<'r>> = Vec::new();
        for (k, ins) in self.gadgets.iter().enumerate() {
            let donde = format!("gadget «{}»", ins.id);
            if !tipo_valido(&ins.id) {
                return Err(format!(
                    "{donde}: an id is lowercase letters, digits and hyphens, starting with a letter"
                ));
            }
            if self.gadgets[..k].iter().any(|o| o.id == ins.id) {
                return Err(format!("{donde}: the id is used twice"));
            }
            let g = reg.buscar(&ins.tipo).ok_or_else(|| {
                format!(
                    "{donde}: no gadget «{}» here; there are: {}",
                    ins.tipo,
                    reg.nombres().join(", ")
                )
            })?;
            let (t0, t1) = (
                tiempos
                    .resolver(&ins.entra)
                    .map_err(|e| format!("{donde} entra: {e}"))?,
                tiempos
                    .resolver(&ins.sale)
                    .map_err(|e| format!("{donde} sale: {e}"))?,
            );
            if !(t0 >= 0.0 && t0 < t1) {
                return Err(format!(
                    "{donde}: entra ({t0:.3} s) must be ≥ 0 and before sale ({t1:.3} s)"
                ));
            }
            if !(ins.fundido.is_finite() && ins.fundido >= 0.0) {
                return Err(format!("{donde}: fundido ≥ 0"));
            }
            let mut base = Params::defectos(g.params());
            let mut enlaces = Vec::new();
            for (nombre, e) in &ins.params {
                let aqui = format!("{donde} param «{nombre}»");
                let spec = g
                    .params()
                    .iter()
                    .find(|s| s.nombre == nombre)
                    .ok_or_else(|| {
                        let hay: Vec<&str> = g.params().iter().map(|s| s.nombre).collect();
                        format!(
                            "{aqui}: «{}» has no such param; it has: {}",
                            ins.tipo,
                            hay.join(", ")
                        )
                    })?;
                if e.procedencia.is_some() && !spec.fisico {
                    return Err(format!(
                        "{aqui}: a layout value carries no procedencia (only physical ones do)"
                    ));
                }
                match &e.origen {
                    Origen::Valor(d) => {
                        spec.admite(d).map_err(|m| format!("{aqui}: {m}"))?;
                        if let (Tipo::Token { .. }, Dato::Texto(t)) = (spec.tipo, d) {
                            if cx.paleta.color(t).is_none() {
                                return Err(format!(
                                    "{aqui}: «{t}» is neither a role nor a token of this palette"
                                ));
                            }
                        }
                        if spec.fisico {
                            let pr = e.procedencia.ok_or_else(|| {
                                format!(
                                    "{aqui}: a physical value needs its procedencia (calculado, medido, reportado or supuesto)"
                                )
                            })?;
                            if pr.pide_fuente() && e.fuente.trim().is_empty() {
                                return Err(format!(
                                    "{aqui}: a {pr} value needs its fuente (where the number comes from)"
                                ));
                            }
                        }
                        base.fijar(nombre, d.clone(), e.procedencia);
                    }
                    Origen::De(r) => {
                        if !matches!(spec.tipo, Tipo::Num { .. }) {
                            return Err(format!("{aqui}: only a number can be bound"));
                        }
                        let (id, salida) = r
                            .rsplit_once('.')
                            .ok_or_else(|| format!("{aqui}: de = \"{r}\" is «<id>.<salida>»"))?;
                        let j = items.iter().position(|it| it.id == id).ok_or_else(|| {
                            format!("{aqui}: de = \"{r}\": no earlier gadget «{id}» (a binding reads a gadget above it)")
                        })?;
                        let fuente = items[j].g;
                        if !fuente.salidas().iter().any(|s| s.nombre == salida) {
                            let hay: Vec<&str> =
                                fuente.salidas().iter().map(|s| s.nombre).collect();
                            return Err(format!(
                                "{aqui}: «{id}» has no output «{salida}»; it has: {}",
                                hay.join(", ")
                            ));
                        }
                        enlaces.push((nombre.clone(), j, salida.to_string()));
                    }
                }
            }
            for s in g.params().iter().filter(|s| s.fisico) {
                if !ins.params.contains_key(s.nombre) {
                    return Err(format!(
                        "{donde} param «{}»: a physical value is written with its procedencia, never left to a default",
                        s.nombre
                    ));
                }
            }
            let mut tramos: BTreeMap<String, Vec<Tramo>> = BTreeMap::new();
            for a in &ins.anims {
                let aqui = format!("{donde} anim «{}»", a.param);
                let spec = g
                    .params()
                    .iter()
                    .find(|s| s.nombre == a.param)
                    .ok_or_else(|| format!("{aqui}: no such param"))?;
                if enlaces.iter().any(|(n, _, _)| *n == a.param) {
                    return Err(format!("{aqui}: a bound param is not animated"));
                }
                for x in [a.de, a.a] {
                    spec.admite(&Dato::Num(x))
                        .map_err(|m| format!("{aqui}: {m}"))?;
                }
                let (u0, u1) = (
                    tiempos
                        .resolver(&a.desde)
                        .map_err(|e| format!("{aqui} desde: {e}"))?,
                    tiempos
                        .resolver(&a.hasta)
                        .map_err(|e| format!("{aqui} hasta: {e}"))?,
                );
                if u0 >= u1 {
                    return Err(format!(
                        "{aqui}: desde ({u0:.3} s) must be before hasta ({u1:.3} s)"
                    ));
                }
                tramos.entry(a.param.clone()).or_default().push(Tramo {
                    de: a.de,
                    a: a.a,
                    t0: u0,
                    t1: u1,
                    curva: a.curva,
                });
            }
            if let Some(m) = g.comprobar(&base, cx).into_iter().next() {
                return Err(format!("{donde}: {m}"));
            }
            items.push(Item {
                id: ins.id.clone(),
                g,
                t0,
                t1,
                fundido: ins.fundido,
                base,
                tramos,
                enlaces,
            });
        }
        Ok(Compilada {
            items,
            duracion: tiempos.duracion,
        })
    }

    /// Sets one param's value from the studio (AC6), checked against its
    /// spec. A binding is replaced by the value; the provenance, its source
    /// and note are kept unless given; a physical param new to the file
    /// starts as *supuesto*, written, so the file says it.
    pub fn fijar(
        &mut self,
        reg: &Registro,
        id: &str,
        param: &str,
        d: Dato,
        procedencia: Option<Procedencia>,
    ) -> Result<(), String> {
        let ins = self
            .gadgets
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| format!("no gadget «{id}» in this scene"))?;
        let g = reg
            .buscar(&ins.tipo)
            .ok_or_else(|| format!("no gadget type «{}»", ins.tipo))?;
        let spec = g
            .params()
            .iter()
            .find(|s| s.nombre == param)
            .ok_or_else(|| format!("«{}» has no param «{param}»", ins.tipo))?;
        spec.admite(&d)
            .map_err(|m| format!("gadget «{id}» param «{param}»: {m}"))?;
        let e = ins
            .params
            .entry(param.to_string())
            .or_insert_with(|| Entrada::valor(d.clone()));
        e.origen = Origen::Valor(d);
        if spec.fisico {
            e.procedencia = procedencia
                .or(e.procedencia)
                .or(Some(Procedencia::Supuesto));
        }
        Ok(())
    }

    /// The file's text, canonical (AC5): the same scene, the same bytes.
    pub fn a_toml(&self, reg: &Registro) -> String {
        let mut s = String::from(
            "# The gadgets of this reel (guion-video-creator R-0015). `guion studio` edits\n\
             # this file and the render reads it. Anchors: inicio, fin, a voice line id\n\
             # (voz.toml) or <id>.fin, ± seconds. Physical values carry their procedencia.\n",
        );
        s.push_str("version = 1\n");
        let _ = writeln!(s, "cola = {}", num(self.cola));
        if let Some(d) = self.duracion {
            let _ = writeln!(s, "duracion = {}", num(d));
        }
        for g in &self.gadgets {
            s.push_str("\n[[gadget]]\n");
            let _ = writeln!(s, "id = {}", cadena(&g.id));
            let _ = writeln!(s, "tipo = {}", cadena(&g.tipo));
            let _ = writeln!(s, "entra = {}", cadena(&g.entra));
            let _ = writeln!(s, "sale = {}", cadena(&g.sale));
            if g.fundido != FUNDIDO {
                let _ = writeln!(s, "fundido = {}", num(g.fundido));
            }
            if !g.params.is_empty() {
                s.push_str("\n[gadget.params]\n");
                // the gadget's declared order, then anything it does not declare
                let orden: Vec<&str> = reg
                    .buscar(&g.tipo)
                    .map(|t| t.params().iter().map(|p| p.nombre).collect())
                    .unwrap_or_default();
                let mut claves: Vec<&String> = g.params.keys().collect();
                claves.sort_by_key(|k| orden.iter().position(|o| o == k).unwrap_or(usize::MAX));
                for k in claves {
                    let _ = writeln!(s, "{} = {}", clave(k), entrada_toml(&g.params[k]));
                }
            }
            for a in &g.anims {
                s.push_str("\n[[gadget.anim]]\n");
                let _ = writeln!(s, "param = {}", cadena(&a.param));
                let _ = writeln!(s, "de = {}", num(a.de));
                let _ = writeln!(s, "a = {}", num(a.a));
                let _ = writeln!(s, "desde = {}", cadena(&a.desde));
                let _ = writeln!(s, "hasta = {}", cadena(&a.hasta));
                let _ = writeln!(s, "curva = {}", cadena(a.curva.nombre()));
            }
        }
        s
    }
}

/// A float as TOML: Rust's shortest round-trip form, always with a point.
fn num(x: f64) -> String {
    let s = format!("{x}");
    if s.contains(['.', 'e', 'E', 'i', 'N']) {
        s
    } else {
        format!("{s}.0")
    }
}

/// A TOML basic string.
fn cadena(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => o.push_str("\\r"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(o, "\\u{:04X}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A key: bare when it can be.
fn clave(k: &str) -> String {
    if !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        k.to_string()
    } else {
        cadena(k)
    }
}

fn dato_toml(d: &Dato) -> String {
    match d {
        Dato::Num(x) => num(*x),
        Dato::Texto(s) => cadena(s),
    }
}

fn entrada_toml(e: &Entrada) -> String {
    let simple = e.procedencia.is_none() && e.fuente.is_empty() && e.nota.is_empty();
    match (&e.origen, simple) {
        (Origen::Valor(d), true) => dato_toml(d),
        (origen, _) => {
            let mut partes = vec![match origen {
                Origen::Valor(d) => format!("valor = {}", dato_toml(d)),
                Origen::De(r) => format!("de = {}", cadena(r)),
            }];
            if let Some(p) = e.procedencia {
                partes.push(format!("procedencia = {}", cadena(p.nombre())));
            }
            if !e.fuente.is_empty() {
                partes.push(format!("fuente = {}", cadena(&e.fuente)));
            }
            if !e.nota.is_empty() {
                partes.push(format!("nota = {}", cadena(&e.nota)));
            }
            format!("{{ {} }}", partes.join(", "))
        }
    }
}

struct Item<'r> {
    id: String,
    g: &'r dyn Gadget,
    t0: f64,
    t1: f64,
    fundido: f64,
    base: Params,
    tramos: BTreeMap<String, Vec<Tramo>>,
    enlaces: Vec<(String, usize, String)>,
}

/// One gadget at one instant, as the studio shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Estado {
    pub id: String,
    pub tipo: String,
    /// The clip's opacity (0: off screen).
    pub alfa: f64,
    /// The params it evaluated with (animated and bound).
    pub params: Params,
    pub eventos: Vec<String>,
    pub valores: Vec<Valor>,
    pub avisos: Vec<String>,
}

/// A frame of the scene: what to draw and each gadget's state.
#[derive(Debug, Clone, PartialEq)]
pub struct Cuadro {
    pub prims: Vec<Prim2>,
    pub gadgets: Vec<Estado>,
}

/// A scene resolved against a registry, a voice and a palette: a pure
/// function of time.
pub struct Compilada<'r> {
    items: Vec<Item<'r>>,
    pub duracion: f64,
}

impl Compilada<'_> {
    /// The frame at `t` (AC1: pure; the same `t`, the same frame). Every
    /// gadget is evaluated, on screen or not, so a binding always reads a
    /// value; only clips on screen draw.
    pub fn cuadro(&self, t: f64, cx: &Contexto) -> Cuadro {
        let mut prims = Vec::new();
        let mut estados: Vec<Estado> = Vec::with_capacity(self.items.len());
        for it in &self.items {
            let mut p = it.base.clone();
            for (n, tr) in &it.tramos {
                let pr = p.procedencia(n);
                p.fijar(n, Dato::Num(animar(p.num(n), tr, t)), pr);
            }
            for (n, j, salida) in &it.enlaces {
                if let Some(v) = estados[*j].valores.iter().find(|v| v.nombre == *salida) {
                    p.fijar(n, Dato::Num(v.cantidad), Some(v.procedencia));
                }
            }
            let alfa = opacidad(t, it.t0, it.t1, it.fundido, self.duracion);
            let Salida {
                prims: pr,
                eventos,
                valores,
                avisos,
            } = it.g.eval(t - it.t0, &p, cx);
            if alfa > 0.0 {
                prims.extend(if alfa < 1.0 { con_alfa(pr, alfa) } else { pr });
            }
            estados.push(Estado {
                id: it.id.clone(),
                tipo: it.g.tipo().to_string(),
                alfa,
                params: p,
                eventos,
                valores,
                avisos,
            });
        }
        Cuadro {
            prims,
            gadgets: estados,
        }
    }

    /// The frames of a reel at `fps`.
    fn instantes(&self, fps: f64) -> impl Iterator<Item = f64> {
        let n = (self.duracion * fps).round() as usize;
        (0..n).map(move |k| k as f64 / fps)
    }

    /// The clock (AC4): `voz:<id>` per line (as guion-marca writes it);
    /// per clip `<id>:entra` and, if it leaves before the end, `<id>:sale`;
    /// the first frame each gadget event holds on screen, `<id>:<evento>`;
    /// and a series per output, `<id>.<salida>`, over the clip's frames.
    pub fn reloj(&self, voz: Option<&Narration>, fps: f64, cx: &Contexto) -> Result<Clock, String> {
        let mut c = Clock::new();
        for l in voz.map(|v| v.lines.as_slice()).unwrap_or_default() {
            c = c
                .event(&format!("voz:{}", l.id), l.t0)
                .map_err(|e| e.to_string())?;
        }
        let ts: Vec<f64> = self.instantes(fps).collect();
        let cuadros: Vec<Cuadro> = ts.iter().map(|&t| self.cuadro(t, cx)).collect();
        for (k, it) in self.items.iter().enumerate() {
            c = c
                .event(&format!("{}:entra", it.id), it.t0)
                .map_err(|e| e.to_string())?;
            if it.t1 < self.duracion {
                c = c
                    .event(&format!("{}:sale", it.id), it.t1)
                    .map_err(|e| e.to_string())?;
            }
            let mut vistos: Vec<String> = Vec::new();
            let mut series: Vec<(String, f64, Vec<f64>)> = Vec::new();
            for (t, q) in ts.iter().zip(&cuadros) {
                let e = &q.gadgets[k];
                if e.alfa <= 0.0 {
                    continue;
                }
                for ev in &e.eventos {
                    if !vistos.contains(ev) {
                        vistos.push(ev.clone());
                        c = c
                            .event(&format!("{}:{ev}", it.id), *t)
                            .map_err(|e| e.to_string())?;
                    }
                }
                for v in &e.valores {
                    match series.iter_mut().find(|(n, _, _)| *n == v.nombre) {
                        Some((_, _, xs)) => xs.push(v.cantidad),
                        None => series.push((v.nombre.clone(), *t, vec![v.cantidad])),
                    }
                }
            }
            for (n, t0, xs) in series {
                c = c
                    .series(&format!("{}.{n}", it.id), t0, 1.0 / fps, xs)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(c)
    }

    /// Every aviso of every frame a clip is on screen, each once, with the
    /// first instant it holds: what `guion studio --check` fails on.
    pub fn avisos(&self, fps: f64, cx: &Contexto) -> Vec<String> {
        let mut vistos: Vec<(String, String)> = Vec::new();
        let mut out = Vec::new();
        for t in self.instantes(fps) {
            for e in self.cuadro(t, cx).gadgets {
                if e.alfa <= 0.0 {
                    continue;
                }
                for a in e.avisos {
                    if !vistos.iter().any(|(id, m)| *id == e.id && *m == a) {
                        out.push(format!("t = {t:.2} s · {}: {a}", e.id));
                        vistos.push((e.id.clone(), a));
                    }
                }
            }
        }
        out
    }
}
