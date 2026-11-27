//! A narrated reel's voice: `voz.toml` → `voz.wav` + `voz.json` (R-0012 AC10).
//!
//! The format is the one the first studio wrote (rotorf-sico's
//! `tools/narra.py`, then its `narra` binary): Spanish keys in, the same two
//! files out, byte for byte. Piper and the line-by-line clock are R-0007's
//! [`narrate_lines`]; this module only reads `voz.toml` (unknown keys fail,
//! by name) and writes `voz.json` exactly as Python's
//! `json.dumps(..., ensure_ascii=False, indent=1)` did, since the scenes
//! and their times were measured on those bytes.
//!
//! ```toml
//! [voz]
//! modelo = "en-gb-alan-low"   # <voices>/<modelo>/<modelo>.onnx
//! inicio = 0.6                # s before the first line
//! pausa = 0.45                # s between lines
//! lento = 1.0                 # Piper's length_scale
//!
//! [[linea]]
//! id = "gancho"
//! texto = "…"                 # what is said
//! subtitulo = "…"             # what is read (default: texto)
//! pausa = 1.2                 # optional: extra silence after
//! antes = 0.8                 # optional: extra silence before
//! desde = 2                   # optional: the subtitle enters on sentence 2
//! voz = "profe"               # optional: spoken by [voces.profe] (R-0018)
//!
//! [voces.profe]               # optional: another voice in the same reel
//! modelo = "es_ES-davefx-medium"
//! lento = 1.0                 # default: [voz] lento
//! ```
//!
//! A line without `voz` is spoken by `[voz]`. Without `[voces]` the output is
//! the same, byte for byte, as before R-0018: `voz.json` does not name the
//! voice of each line (a scene knows its lines by `id`).
//!
//! **The natural voice (R-0019).** `prosodia` in `[voz]` opts in:
//!
//! ```toml
//! [voz]
//! modelo = "es-carlfm-x-low"
//! prosodia = "natural"        # «C»: noise 0.75, noise_w 0.9, silence 0.35 s
//!                             # ("determinista": R-0007's 0/0, explicit)
//! ruido = 0.75                # optional: noise_scale (also in [voces.x])
//! ruido_w = 0.9               # optional: noise_w
//! silencio = 0.35             # optional: Piper's pause between sentences, s
//! ```
//!
//! An account's `cuenta.toml` `[voz.prosodia]` replaces «C»'s numbers for its
//! natural posts; `voz.toml`'s own replace the account's. Each line is then
//! spoken once and kept in `voz.tomas/` (commit it: a take with noise cannot
//! be made again), and `voz.wav` and `voz.json` are derived from the takes,
//! so a re-run speaks nothing and gives the same bytes. Without `prosodia`
//! the voice is the legacy one, byte for byte.
//!
//! Nothing is written unless everything was synthesized and checked.

use std::path::{Path, PathBuf};

use guion_audio::{
    narrate_cast, narrate_utterances, Line, Narration, NarrationEngine, Pacing, PiperEngine,
    Prosody, SilenceRule, TakeNarration, TakeStore, Utterance,
};

const SR: u32 = 48_000;

/// The folder, inside the post, that keeps the voice's takes (R-0019 AC5).
pub const TOMAS: &str = "voz.tomas";

/// The Piper numbers a table may set (R-0019 AC1); `None` keeps the layer
/// below.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Ajustes {
    ruido: Option<f64>,
    ruido_w: Option<f64>,
    silencio: Option<f64>,
}

/// `[voz] prosodia`: which voice the reel asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Preset {
    /// «C», the owner's natural voice (and the account's numbers).
    Natural,
    /// R-0007's 0/0, asked for explicitly.
    Determinista,
}

/// Una línea del guion de la voz, con lo que es solo de la casa.
struct Linea {
    linea: Line,
    subtitulo: String,
    desde: usize,
    /// The voice that speaks it: `None` is `[voz]`, `Some(i)` is `voces[i]`.
    voz: Option<usize>,
}

/// A voice of the reel's cast (R-0018): `[voces.<nombre>]`.
struct Reparto {
    nombre: String,
    modelo: String,
    lento: f64,
    ajustes: Ajustes,
}

struct Guion {
    modelo: String,
    lento: f64,
    pacing: Pacing,
    voces: Vec<Reparto>,
    lineas: Vec<Linea>,
    /// `None`: the legacy voice (R-0019 AC3).
    prosodia: Option<Preset>,
    ajustes: Ajustes,
}

/// Makes the natural voice of a model with a prosody (Piper, or a fake in
/// the tests).
type Hacer<'a> = dyn Fn(&str, Prosody) -> Result<Box<dyn guion_audio::Voice>, String> + 'a;

/// Where the voices live and which Piper speaks them.
pub struct Voice {
    /// The folder with `<model>/<model>.onnx`.
    pub voices: PathBuf,
    /// The Piper executable; `None`: `GUION_PIPER`, then `PATH`.
    pub piper: Option<PathBuf>,
    /// The model `reel.toml` expects; `voz.toml` must name the same one.
    pub model: Option<String>,
}

/// `voz.toml` in `post` → `voz.wav` and `voz.json` next to it.
pub fn narrate(post: &Path, voice: &Voice) -> Result<(), String> {
    narrate_con(post, voice, &|modelo, p| {
        piper_natural(voice, modelo, p).map(|v| Box::new(v) as Box<dyn guion_audio::Voice>)
    })
}

/// [`narrate`] with the natural voices made by `hacer` (R-0019).
fn narrate_con(post: &Path, voice: &Voice, hacer: &Hacer) -> Result<(), String> {
    let toml_path = post.join("voz.toml");
    let fuente =
        std::fs::read_to_string(&toml_path).map_err(|e| format!("{}: {e}", toml_path.display()))?;
    let g = leer_guion(&fuente).map_err(|e| format!("{}: {e}", toml_path.display()))?;
    if let Some(m) = voice.model.as_ref().filter(|m| **m != g.modelo) {
        return Err(format!(
            "reel.toml asks for the voice «{m}» and voz.toml names «{}»",
            g.modelo
        ));
    }
    let (voz, tomas) = match g.prosodia {
        None => (legado(&g, voice)?, None),
        Some(preset) => {
            let (r, store) = natural(post, &g, preset, hacer)?;
            (r.narration, Some((r.takes, store)))
        }
    };
    for (l, t) in g.lineas.iter().zip(&voz.lines) {
        if !(1..=t.sentences.len()).contains(&l.desde) {
            return Err(format!(
                "«{}» tiene desde = {} y {} frases",
                l.linea.id,
                l.desde,
                t.sentences.len()
            ));
        }
    }
    // todo medido y revisado: ahora sí se escribe
    let json = voz_json(&g, &voz);
    voz.write_wav(&post.join("voz.wav"))
        .map_err(|e| e.to_string())?;
    escribir(&post.join("voz.json"), &json)?;
    println!(
        "voice: {} lines, {:.2} s → {}",
        voz.lines.len(),
        voz.duration(),
        post.join("voz.wav").display()
    );
    match tomas {
        None => println!(
            "  legacy voice (no `prosodia` in voz.toml: deterministic, line by line); \
             `prosodia = \"natural\"` opts in (R-0019)"
        ),
        Some((usadas, store)) => {
            let quitadas = store.settle(&usadas).map_err(|e| e.to_string())?;
            let habladas = usadas.iter().filter(|u| !u.cached).count();
            println!(
                "  takes: {habladas} spoken, {} kept, {} removed → {} (commit it with the post)",
                usadas.len() - habladas,
                quitadas.len(),
                store.dir().display()
            );
        }
    }
    Ok(())
}

/// The legacy voice (R-0007, R-0018): deterministic Piper, line by line,
/// exactly as before R-0019.
fn legado(g: &Guion, voice: &Voice) -> Result<Narration, String> {
    // every voice is found before anything is spoken (R-0018 AC2)
    let principal = piper(voice, &g.modelo, g.lento)?;
    let reparto = g
        .voces
        .iter()
        .map(|r| piper(voice, &r.modelo, r.lento))
        .collect::<Result<Vec<_>, _>>()?;
    let motores: Vec<&dyn NarrationEngine> = g
        .lineas
        .iter()
        .map(|l| match l.voz {
            None => &principal as &dyn NarrationEngine,
            Some(i) => &reparto[i],
        })
        .collect();
    let lineas: Vec<Line> = g.lineas.iter().map(|l| l.linea.clone()).collect();
    narrate_cast(&lineas, &motores, &g.pacing, &SilenceRule::PIPER).map_err(|e| e.to_string())
}

/// The natural voice (R-0019): every voice made first (R-0018 AC2), then
/// each line spoken once and kept in [`TOMAS`].
fn natural(
    post: &Path,
    g: &Guion,
    preset: Preset,
    hacer: &Hacer,
) -> Result<(TakeNarration, TakeStore), String> {
    let cuenta = match preset {
        Preset::Natural => ajustes_de_cuenta(post)?,
        Preset::Determinista => Ajustes::default(),
    };
    let principal = hacer(
        &g.modelo,
        prosodia(preset, cuenta, g.ajustes, Ajustes::default(), g.lento)?,
    )?;
    let reparto = g
        .voces
        .iter()
        .map(|r| {
            hacer(
                &r.modelo,
                prosodia(preset, cuenta, g.ajustes, r.ajustes, r.lento)?,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let utterances: Vec<Utterance> = g
        .lineas
        .iter()
        .enumerate()
        .map(|(i, l)| Utterance {
            lines: i..i + 1,
            voice: match l.voz {
                None => principal.as_ref(),
                Some(k) => reparto[k].as_ref(),
            },
        })
        .collect();
    let lineas: Vec<Line> = g.lineas.iter().map(|l| l.linea.clone()).collect();
    let dichas: Vec<String> = lineas.iter().map(|l| l.text.clone()).collect();
    let store = TakeStore::new(post.join(TOMAS));
    let r = narrate_utterances(&lineas, &dichas, &utterances, &g.pacing, &store)
        .map_err(|e| e.to_string())?;
    Ok((r, store))
}

/// The prosody of one voice (R-0019 AC1, AC2): the preset, then (natural
/// only) the account's numbers, then `[voz]`'s, then the cast voice's.
fn prosodia(
    preset: Preset,
    cuenta: Ajustes,
    voz: Ajustes,
    reparto: Ajustes,
    lento: f64,
) -> Result<Prosody, String> {
    let (mut p, capas) = match preset {
        Preset::Natural => (Prosody::NATURAL, vec![cuenta, voz, reparto]),
        Preset::Determinista => (Prosody::DETERMINISTIC, vec![voz, reparto]),
    };
    for a in capas {
        p.noise_scale = a.ruido.unwrap_or(p.noise_scale);
        p.noise_w = a.ruido_w.unwrap_or(p.noise_w);
        p.sentence_silence = a.silencio.unwrap_or(p.sentence_silence);
    }
    p.length_scale = lento;
    p.check()?;
    Ok(p)
}

/// `[voz.prosodia]` of the account the post belongs to: the nearest
/// `cuenta.toml` above it (R-0019 AC2). None: «C» as it is.
fn ajustes_de_cuenta(post: &Path) -> Result<Ajustes, String> {
    let abs = post
        .canonicalize()
        .map_err(|e| format!("{}: {e}", post.display()))?;
    let Some(raiz) = abs
        .ancestors()
        .find(|d| d.join(guion_cuenta::contrato::ARCHIVO).is_file())
    else {
        return Ok(Ajustes::default());
    };
    let c = guion_cuenta::contrato::Cuenta::load(raiz)?;
    Ok(c.voz
        .prosodia
        .map(|p| Ajustes {
            ruido: p.ruido,
            ruido_w: p.ruido_w,
            silencio: p.silencio,
        })
        .unwrap_or_default())
}

/// The deterministic Piper for `modelo` (`<voices>/<modelo>/<modelo>.onnx`).
fn piper(voice: &Voice, modelo: &str, lento: f64) -> Result<PiperEngine, String> {
    let mut p = PiperEngine::deterministic(onnx(voice, modelo)?)
        .length_scale(lento)
        .sentence_silence(0.25);
    if let Some(b) = &voice.piper {
        p = p.binary(b.clone());
    }
    Ok(p)
}

/// The natural Piper for `modelo` with `p` (R-0019).
fn piper_natural(voice: &Voice, modelo: &str, p: Prosody) -> Result<PiperEngine, String> {
    let mut e = PiperEngine::with_prosody(onnx(voice, modelo)?, p);
    if let Some(b) = &voice.piper {
        e = e.binary(b.clone());
    }
    Ok(e)
}

/// `<voices>/<modelo>/<modelo>.onnx`, which must exist.
fn onnx(voice: &Voice, modelo: &str) -> Result<String, String> {
    let onnx = voice.voices.join(modelo).join(format!("{modelo}.onnx"));
    if !onnx.exists() {
        return Err(format!("no voice at {}", onnx.display()));
    }
    Ok(onnx.display().to_string())
}

/// `[voces.<nombre>]`: the reel's other voices, in name order (R-0018 AC1).
fn leer_voces(raiz: &toml::Table, lento: f64) -> Result<Vec<Reparto>, String> {
    let Some(v) = raiz.get("voces") else {
        return Ok(Vec::new());
    };
    let v = v.as_table().ok_or("[voces] no es una tabla")?;
    let mut voces = Vec::new();
    for (nombre, t) in v {
        let donde = format!("[voces.{nombre}]");
        let t = t.as_table().ok_or(format!("{donde}: no es una tabla"))?;
        claves(
            t,
            &donde,
            &["modelo", "lento", "ruido", "ruido_w", "silencio"],
        )?;
        let modelo = t
            .get("modelo")
            .and_then(|m| m.as_str())
            .ok_or(format!("{donde}: falta modelo (el nombre de una voz)"))?
            .to_string();
        voces.push(Reparto {
            nombre: nombre.clone(),
            modelo,
            lento: numero(t, "lento", &donde)?.unwrap_or(lento),
            ajustes: leer_ajustes(t, &donde)?,
        });
    }
    Ok(voces)
}

/// `ruido`, `ruido_w` and `silencio` of a table: numbers ≥ 0 (R-0019 AC1).
fn leer_ajustes(t: &toml::Table, donde: &str) -> Result<Ajustes, String> {
    let n = |k: &str| -> Result<Option<f64>, String> {
        match numero(t, k, donde)? {
            Some(x) if !(x.is_finite() && x >= 0.0) => {
                Err(format!("{donde}: {k} = {x}: debe ser un número ≥ 0"))
            }
            x => Ok(x),
        }
    };
    Ok(Ajustes {
        ruido: n("ruido")?,
        ruido_w: n("ruido_w")?,
        silencio: n("silencio")?,
    })
}

/// A table's prosody keys must fit `[voz] prosodia` (R-0019 AC1): none
/// without it, no noise with `determinista`, a `lento` > 0.
fn revisar_prosodia(
    prosodia: Option<Preset>,
    a: &Ajustes,
    lento: f64,
    donde: &str,
) -> Result<(), String> {
    let algo = [a.ruido, a.ruido_w, a.silencio].iter().any(Option::is_some);
    match prosodia {
        None if algo => Err(format!(
            "{donde}: ruido, ruido_w y silencio piden [voz] prosodia = \"natural\" o \
             \"determinista\" (sin prosodia, la voz es la de antes)"
        )),
        Some(Preset::Determinista) if a.ruido.is_some() || a.ruido_w.is_some() => Err(format!(
            "{donde}: prosodia = \"determinista\" fija ruido y ruido_w en 0; quítalos o usa \
             \"natural\""
        )),
        Some(_) if !(lento.is_finite() && lento > 0.0) => {
            Err(format!("{donde}: lento = {lento}: debe ser > 0"))
        }
        _ => Ok(()),
    }
}

fn leer_guion(fuente: &str) -> Result<Guion, String> {
    let raiz: toml::Table = fuente.parse().map_err(|e: toml::de::Error| e.to_string())?;
    claves(&raiz, "", &["voz", "voces", "linea"])?;
    let v = raiz
        .get("voz")
        .and_then(|v| v.as_table())
        .ok_or("falta la tabla [voz]")?;
    claves(
        v,
        "[voz]",
        &[
            "modelo", "inicio", "pausa", "lento", "prosodia", "ruido", "ruido_w", "silencio",
        ],
    )?;
    let modelo = v
        .get("modelo")
        .and_then(|m| m.as_str())
        .ok_or("[voz]: falta modelo (el nombre de una voz)")?
        .to_string();
    let lento = numero(v, "lento", "[voz]")?.unwrap_or(1.0);
    let pacing = Pacing {
        sample_rate: SR,
        lead_in: numero(v, "inicio", "[voz]")?.unwrap_or(0.6),
        gap: numero(v, "pausa", "[voz]")?.unwrap_or(0.45),
    };
    let prosodia = match v.get("prosodia") {
        None => None,
        Some(p) => Some(match p.as_str() {
            Some("natural") => Preset::Natural,
            Some("determinista") => Preset::Determinista,
            _ => {
                return Err(format!(
                    "[voz]: prosodia = {p}: valen \"natural\" o \"determinista\""
                ))
            }
        }),
    };
    let ajustes = leer_ajustes(v, "[voz]")?;
    revisar_prosodia(prosodia, &ajustes, lento, "[voz]")?;
    let voces = leer_voces(&raiz, lento)?;
    for r in &voces {
        revisar_prosodia(
            prosodia,
            &r.ajustes,
            r.lento,
            &format!("[voces.{}]", r.nombre),
        )?;
    }
    let tabla = raiz
        .get("linea")
        .and_then(|l| l.as_array())
        .ok_or("faltan las líneas ([[linea]])")?;
    let mut lineas = Vec::new();
    for (i, l) in tabla.iter().enumerate() {
        let donde = format!("[[linea]] {}", i + 1);
        let l = l.as_table().ok_or(format!("{donde}: no es una tabla"))?;
        claves(
            l,
            &donde,
            &["id", "texto", "subtitulo", "pausa", "antes", "desde", "voz"],
        )?;
        let id = l
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or(format!("{donde}: falta id"))?;
        let donde = format!("[[linea]] «{id}»");
        let texto = l
            .get("texto")
            .and_then(|x| x.as_str())
            .ok_or(format!("{donde}: falta texto"))?;
        let subtitulo = match l.get("subtitulo") {
            None => texto.to_string(),
            Some(s) => s
                .as_str()
                .ok_or(format!("{donde}: subtitulo no es texto"))?
                .to_string(),
        };
        let desde = match l.get("desde") {
            None => 1,
            Some(d) => d
                .as_integer()
                .filter(|d| *d >= 1)
                .ok_or(format!("{donde}: desde debe ser un entero ≥ 1"))?
                as usize,
        };
        let voz = match l.get("voz") {
            None => None,
            Some(n) => {
                let n = n.as_str().ok_or(format!("{donde}: voz no es texto"))?;
                let i = voces.iter().position(|r| r.nombre == n).ok_or_else(|| {
                    let hay: Vec<&str> = voces.iter().map(|r| r.nombre.as_str()).collect();
                    let hay = if hay.is_empty() {
                        "ninguna".to_string()
                    } else {
                        hay.join(", ")
                    };
                    format!("{donde}: voz = «{n}» no está en [voces] (hay: {hay})")
                })?;
                Some(i)
            }
        };
        let mut linea = Line::new(id, texto);
        linea.before = numero(l, "antes", &donde)?.unwrap_or(0.0);
        linea.after = numero(l, "pausa", &donde)?.unwrap_or(0.0);
        lineas.push(Linea {
            linea,
            subtitulo,
            desde,
            voz,
        });
    }
    Ok(Guion {
        modelo,
        lento,
        pacing,
        voces,
        lineas,
        prosodia,
        ajustes,
    })
}

fn claves(t: &toml::Table, donde: &str, validas: &[&str]) -> Result<(), String> {
    for k in t.keys() {
        if !validas.contains(&k.as_str()) {
            let en = if donde.is_empty() {
                String::new()
            } else {
                format!("{donde}: ")
            };
            return Err(format!(
                "{en}clave desconocida «{k}» (valen: {})",
                validas.join(", ")
            ));
        }
    }
    Ok(())
}

/// Un número de TOML, entero o con punto (como `float()` en Python).
fn numero(t: &toml::Table, k: &str, donde: &str) -> Result<Option<f64>, String> {
    match t.get(k) {
        None => Ok(None),
        Some(toml::Value::Float(x)) => Ok(Some(*x)),
        Some(toml::Value::Integer(n)) => Ok(Some(*n as f64)),
        Some(_) => Err(format!("{donde}: {k} debe ser un número")),
    }
}

/// `round(x, 4)` de Python: redondeo correcto del valor binario exacto, con
/// empates al par (el formateo de Rust hace lo mismo).
fn redondo(x: f64) -> f64 {
    format!("{x:.4}")
        .parse()
        .expect("un número formateado se vuelve a leer")
}

/// El `voz.json` que lee la escena, idéntico byte por byte al de
/// `json.dumps(..., ensure_ascii=False, indent=1)` de `tools/narra.py`.
fn voz_json(g: &Guion, voz: &Narration) -> String {
    let lineas = g
        .lineas
        .iter()
        .zip(&voz.lines)
        .map(|(l, t)| {
            let frases = t
                .sentences
                .iter()
                .map(|f| {
                    V::Obj(vec![
                        ("texto", V::Txt(f.text.clone())),
                        ("t0", V::Num(redondo(f.t0))),
                        ("t1", V::Num(redondo(f.t1))),
                    ])
                })
                .collect();
            let mut campos = vec![
                ("id", V::Txt(t.id.clone())),
                ("texto", V::Txt(t.text.clone())),
                ("subtitulo", V::Txt(l.subtitulo.clone())),
                ("t0", V::Num(redondo(t.t0))),
                ("t1", V::Num(redondo(t.t1))),
                ("frases", V::Lista(frases)),
            ];
            if l.desde > 1 {
                campos.push(("desde", V::Ent(l.desde as i64)));
            }
            V::Obj(campos)
        })
        .collect();
    let raiz = V::Obj(vec![
        ("modelo", V::Txt(g.modelo.clone())),
        ("duracion_voz", V::Num(redondo(voz.duration()))),
        ("lineas", V::Lista(lineas)),
    ]);
    let mut s = String::new();
    raiz.escribir(&mut s, 0);
    s.push('\n');
    s
}

/// Lo justo de JSON para escribir como Python (`indent=1`, sin escapar lo que no es ASCII).
enum V {
    Num(f64),
    Ent(i64),
    Txt(String),
    Lista(Vec<V>),
    Obj(Vec<(&'static str, V)>),
}

impl V {
    fn escribir(&self, s: &mut String, nivel: usize) {
        let sangria = |s: &mut String, n: usize| {
            s.push('\n');
            s.push_str(&" ".repeat(n));
        };
        match self {
            V::Num(x) => s.push_str(&repr(*x)),
            V::Ent(n) => s.push_str(&n.to_string()),
            V::Txt(t) => cadena(s, t),
            V::Lista(v) if v.is_empty() => s.push_str("[]"),
            V::Lista(v) => {
                s.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    sangria(s, nivel + 1);
                    x.escribir(s, nivel + 1);
                }
                sangria(s, nivel);
                s.push(']');
            }
            V::Obj(v) if v.is_empty() => s.push_str("{}"),
            V::Obj(v) => {
                s.push('{');
                for (i, (k, x)) in v.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    sangria(s, nivel + 1);
                    cadena(s, k);
                    s.push_str(": ");
                    x.escribir(s, nivel + 1);
                }
                sangria(s, nivel);
                s.push('}');
            }
        }
    }
}

/// `repr(float)` de Python: los dígitos más cortos que vuelven al mismo
/// número, con `.0` si es entero y exponente fuera de [1e-4, 1e16).
fn repr(x: f64) -> String {
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        // 1e-05, 1.5e+16: la mantisa más corta y el exponente con al menos dos cifras
        let e = format!("{x:e}");
        let (m, exp) = e.split_once('e').expect("formato e");
        let exp: i32 = exp.parse().expect("exponente");
        return format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let s = format!("{x}");
    if s.contains('.') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}

fn cadena(s: &mut String, t: &str) {
    s.push('"');
    for c in t.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            '\u{08}' => s.push_str("\\b"),
            '\u{0c}' => s.push_str("\\f"),
            c if (c as u32) < 0x20 => s.push_str(&format!("\\u{:04x}", c as u32)),
            c => s.push(c),
        }
    }
    s.push('"');
}

/// Sin salida a medias: se escribe al lado y se renombra.
fn escribir(ruta: &Path, texto: &str) -> Result<(), String> {
    let tmp = ruta.with_extension("json.parcial");
    std::fs::write(&tmp, texto).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, ruta).map_err(|e| format!("{}: {e}", ruta.display()))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn repr_como_python() {
        for (x, s) in [
            (0.5, "0.5"),
            (13.0, "13.0"),
            (87.464, "87.464"),
            (0.0, "0.0"),
            (1e-5, "1e-05"),
            (2e16, "2e+16"),
        ] {
            assert_eq!(repr(x), s);
        }
        assert_eq!(redondo(0.03125), 0.0312);
        assert_eq!(redondo(12.34565), 12.3456);
    }

    /// R-0018 AC1: a line names a voice of `[voces]`; one without `voz` is
    /// spoken by `[voz]`, and a cast voice without `lento` takes `[voz]`'s.
    #[test]
    fn reparto_por_linea() {
        let g = leer_guion(concat!(
            "[voz]\nmodelo = \"maistro\"\nlento = 1.1\n",
            "[voces.profe]\nmodelo = \"es_ES-davefx-medium\"\n",
            "[voces.coro]\nmodelo = \"c\"\nlento = 0.9\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Ya.\"\n",
            "[[linea]]\nid = \"b\"\ntexto = \"Se fundamenta.\"\nvoz = \"profe\"\n",
        ))
        .unwrap();
        assert_eq!(g.lineas[0].voz, None);
        let i = g.lineas[1].voz.unwrap();
        assert_eq!(g.voces[i].nombre, "profe");
        assert_eq!(g.voces[i].modelo, "es_ES-davefx-medium");
        assert_eq!(g.voces[i].lento, 1.1);
        let coro = g.voces.iter().find(|r| r.nombre == "coro").unwrap();
        assert_eq!(coro.lento, 0.9);
    }

    /// R-0018 AC2: an unknown voice, a cast voice without a model or with an
    /// unknown key fail by name.
    #[test]
    fn reparto_invalido_falla_por_nombre() {
        let e = leer_guion(concat!(
            "[voz]\nmodelo = \"m\"\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\nvoz = \"profe\"\n",
        ))
        .err()
        .unwrap();
        assert!(
            e.contains("«profe»") && e.contains("«a»") && e.contains("ninguna"),
            "{e}"
        );
        let e = leer_guion(concat!(
            "[voz]\nmodelo = \"m\"\n[voces.profe]\nlento = 1\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\n",
        ))
        .err()
        .unwrap();
        assert!(e.contains("[voces.profe]") && e.contains("modelo"), "{e}");
        let e = leer_guion(concat!(
            "[voz]\nmodelo = \"m\"\n[voces.profe]\nmodelo = \"p\"\nhablante = 0\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\n",
        ))
        .err()
        .unwrap();
        assert!(e.contains("[voces.profe]") && e.contains("hablante"), "{e}");
    }

    /// R-0018 AC2: a voice whose model is missing fails before anything is
    /// spoken or written.
    #[test]
    fn una_voz_que_falta_no_escribe_nada() {
        let dir = std::env::temp_dir().join(format!("guion-r0018-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("voces/m")).unwrap();
        std::fs::write(dir.join("voces/m/m.onnx"), b"").unwrap();
        std::fs::write(
            dir.join("voz.toml"),
            concat!(
                "[voz]\nmodelo = \"m\"\n[voces.profe]\nmodelo = \"no-existe\"\n",
                "[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\nvoz = \"profe\"\n",
            ),
        )
        .unwrap();
        let v = Voice {
            voices: dir.join("voces"),
            piper: None,
            model: None,
        };
        let e = narrate(&dir, &v).unwrap_err();
        assert!(e.contains("no-existe"), "{e}");
        assert!(!dir.join("voz.wav").exists() && !dir.join("voz.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn claves_desconocidas_fallan() {
        let e = leer_guion(
            "[voz]\nmodelo = \"m\"\n[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\ndsede = 2\n",
        )
        .err()
        .unwrap();
        assert!(e.contains("dsede") && e.contains("[[linea]] 1"), "{e}");
        assert!(leer_guion(
            "[voz]\nmodelo = \"m\"\npausa = 1\n[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\n"
        )
        .is_ok());
    }

    // ---------- R-0019: the natural voice ----------

    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use guion_audio::{split_sentences, AudioError, Pcm16, TAKE_INDEX};

    /// Speaks like Piper with noise: 0.2 s per word at a level that changes
    /// on every call, and exact zeros between sentences. (A sibling of the
    /// fake in guion-audio's `tests/takes.rs`: a test crate cannot share it.)
    struct Ruidosa {
        llamadas: Rc<Cell<usize>>,
        silencio: f64,
        id: String,
    }

    impl guion_audio::Voice for Ruidosa {
        fn identity(&self) -> Result<String, AudioError> {
            Ok(self.id.clone())
        }
        fn take(&self, text: &str) -> Result<Pcm16, AudioError> {
            let n = self.llamadas.get() + 1;
            self.llamadas.set(n);
            let nivel = 6000 + 41 * n as i16;
            let mut out = Vec::new();
            let frases = split_sentences(text);
            for (i, f) in frases.iter().enumerate() {
                for _ in f.split_whitespace() {
                    out.extend((0..3200).map(|k| if (k / 8) % 2 == 0 { nivel } else { -nivel }));
                }
                if i + 1 < frases.len() {
                    out.resize(out.len() + (self.silencio * 48_000.0) as usize, 0);
                }
            }
            Ok(Pcm16 {
                sample_rate: 48_000,
                channels: 1,
                samples: out,
            })
        }
        fn sentence_silence(&self) -> f64 {
            self.silencio
        }
    }

    /// A temp post with this `voz.toml`, and its voices folder.
    fn post(nombre: &str, voz_toml: &str) -> (PathBuf, Voice) {
        let d = std::env::temp_dir().join(format!("guion-r0019-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("post")).unwrap();
        std::fs::write(d.join("post/voz.toml"), voz_toml).unwrap();
        let v = Voice {
            voices: d.join("voces"),
            piper: None,
            model: None,
        };
        (d, v)
    }

    /// A factory of noisy voices that records the prosody each was asked for.
    fn fabrica(
        llamadas: &Rc<Cell<usize>>,
        pedidas: &Rc<RefCell<Vec<(String, Prosody)>>>,
    ) -> impl Fn(&str, Prosody) -> Result<Box<dyn guion_audio::Voice>, String> {
        let (llamadas, pedidas) = (llamadas.clone(), pedidas.clone());
        move |modelo: &str, p: Prosody| {
            pedidas.borrow_mut().push((modelo.to_string(), p));
            Ok(Box::new(Ruidosa {
                llamadas: llamadas.clone(),
                silencio: p.sentence_silence,
                id: format!("{modelo} {p:?}"),
            }) as Box<dyn guion_audio::Voice>)
        }
    }

    const NATURAL: &str =
        "[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\ninicio = 0.5\npausa = 0.3\n\n\
        [[linea]]\nid = \"a\"\ntexto = \"Uno dos. Tres.\"\nsubtitulo = \"Uno.\"\ndesde = 2\n\n\
        [[linea]]\nid = \"b\"\ntexto = \"Cuatro.\"\nantes = 0.2\n";

    /// R-0019 AC1: the preset, the account, `[voz]` and the cast voice, in
    /// that order; `determinista` ignores the account.
    #[test]
    fn la_prosodia_va_por_capas() {
        let cuenta = Ajustes {
            ruido_w: Some(0.6),
            silencio: Some(0.4),
            ..Ajustes::default()
        };
        let voz = Ajustes {
            ruido_w: Some(0.5),
            ..Ajustes::default()
        };
        let reparto = Ajustes {
            ruido: Some(0.3),
            ..Ajustes::default()
        };
        let p = prosodia(
            Preset::Natural,
            Ajustes::default(),
            Ajustes::default(),
            Ajustes::default(),
            1.0,
        )
        .unwrap();
        assert_eq!(p, Prosody::NATURAL);
        let p = prosodia(Preset::Natural, cuenta, voz, reparto, 1.1).unwrap();
        assert_eq!(
            (p.noise_scale, p.noise_w, p.length_scale, p.sentence_silence),
            (0.3, 0.5, 1.1, 0.4)
        );
        let d = prosodia(
            Preset::Determinista,
            cuenta,
            Ajustes::default(),
            Ajustes::default(),
            1.0,
        )
        .unwrap();
        assert_eq!(d, Prosody::DETERMINISTIC);
    }

    /// R-0019 AC1: what does not fit fails by table and key.
    #[test]
    fn la_prosodia_invalida_falla_por_nombre() {
        let base = "[[linea]]\nid = \"a\"\ntexto = \"Hi.\"\n";
        let e = |voz: &str| leer_guion(&format!("{voz}\n{base}")).err().unwrap();
        let m = e("[voz]\nmodelo = \"m\"\nprosodia = \"suave\"");
        assert!(m.contains("prosodia") && m.contains("natural"), "{m}");
        let m = e("[voz]\nmodelo = \"m\"\nruido = 0.5");
        assert!(m.contains("[voz]") && m.contains("prosodia"), "{m}");
        let m = e("[voz]\nmodelo = \"m\"\nprosodia = \"determinista\"\nruido_w = 0.5");
        assert!(m.contains("determinista") && m.contains("ruido"), "{m}");
        let m = e("[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\nruido = -1");
        assert!(m.contains("ruido = -1"), "{m}");
        let m = e("[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\nlento = 0");
        assert!(m.contains("lento"), "{m}");
        let m = e("[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\n[voces.profe]\nmodelo = \"p\"\nruido = \"x\"");
        assert!(m.contains("[voces.profe]") && m.contains("ruido"), "{m}");
        // the legacy file reads as before
        let g = leer_guion(&format!("[voz]\nmodelo = \"m\"\n{base}")).unwrap();
        assert_eq!(g.prosodia, None);
    }

    /// R-0019 AC4, AC5: the natural voice is spoken once; a re-run, a
    /// subtitle or a pause edited speak nothing and give the same bytes;
    /// a text edited speaks only its line; the takes live in the post.
    #[test]
    fn la_voz_natural_se_graba_una_vez() {
        let (d, v) = post("una-vez", NATURAL);
        let p = d.join("post");
        let llamadas = Rc::new(Cell::new(0));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        let hacer = fabrica(&llamadas, &pedidas);
        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.get(), 2);
        assert_eq!(pedidas.borrow()[0].1, Prosody::NATURAL);
        let wav = std::fs::read(p.join("voz.wav")).unwrap();
        let json = std::fs::read_to_string(p.join("voz.json")).unwrap();
        assert!(json.contains("\"subtitulo\": \"Uno.\""), "{json}");

        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.get(), 2, "a re-run speaks nothing");
        assert_eq!(std::fs::read(p.join("voz.wav")).unwrap(), wav);
        assert_eq!(std::fs::read_to_string(p.join("voz.json")).unwrap(), json);

        let editado = NATURAL
            .replace("subtitulo = \"Uno.\"", "subtitulo = \"Tres.\"")
            .replace("antes = 0.2", "antes = 0.6");
        std::fs::write(p.join("voz.toml"), &editado).unwrap();
        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.get(), 2, "subtitles and pauses are not takes");

        let tomas = p.join(TOMAS);
        let antes: Vec<String> = std::fs::read_dir(&tomas)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        std::fs::write(
            p.join("voz.toml"),
            editado.replace("Cuatro.", "Cuatro y cinco."),
        )
        .unwrap();
        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.get(), 3, "only the edited line is spoken again");
        let despues: Vec<String> = std::fs::read_dir(&tomas)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            despues.iter().filter(|n| n.ends_with(".wav")).count(),
            2,
            "the old take of «b» is gone: {antes:?} → {despues:?}"
        );
        assert!(tomas.join(".gitignore").is_file() && tomas.join(TAKE_INDEX).is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// R-0019 AC2: the account's `[voz.prosodia]` reaches its natural posts;
    /// `voz.toml`'s own numbers win.
    #[test]
    fn la_cuenta_pone_su_voz_natural() {
        use guion_cuenta::nueva::{plan, Respuestas};
        let (d, v) = post("cuenta", NATURAL);
        let mut r = Respuestas::con_defaults("Cuenta", "tema", "revisor", "materia");
        r.fecha = "2026-11-26".into();
        let contrato = plan(&r, "")
            .unwrap()
            .archivos
            .into_iter()
            .find(|a| a.ruta == "cuenta.toml")
            .unwrap()
            .texto
            .replace(
                "\n[marca]\n",
                "\n[voz.prosodia]\nruido_w = 0.6\nsilencio = 0.4\n\n[marca]\n",
            );
        std::fs::write(d.join("cuenta.toml"), contrato).unwrap();
        let llamadas = Rc::new(Cell::new(0));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        let p = d.join("post");
        narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap();
        let pedida = pedidas.borrow()[0].1;
        assert_eq!((pedida.noise_w, pedida.sentence_silence), (0.6, 0.4));
        assert_eq!(
            pedida.noise_scale, 0.75,
            "what the account does not set stays «C»"
        );
        std::fs::write(
            p.join("voz.toml"),
            NATURAL.replace(
                "prosodia = \"natural\"",
                "prosodia = \"natural\"\nruido_w = 0.5",
            ),
        )
        .unwrap();
        narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap();
        assert_eq!(pedidas.borrow().last().unwrap().1.noise_w, 0.5);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// R-0019 AC3: without `prosodia` the legacy path runs (deterministic
    /// Piper): no natural voice is made and no take is written.
    #[test]
    fn sin_prosodia_la_voz_es_la_de_antes() {
        let legado = NATURAL.replace("prosodia = \"natural\"\n", "");
        let (d, v) = post("legado", &legado);
        std::fs::create_dir_all(d.join("voces/m")).unwrap();
        std::fs::write(d.join("voces/m/m.onnx"), b"").unwrap();
        let llamadas = Rc::new(Cell::new(0));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        let p = d.join("post");
        // a fake model: the legacy Piper fails, but it is the one that ran
        let v = Voice {
            piper: Some(d.join("no-hay-piper")),
            ..v
        };
        assert!(narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).is_err());
        assert!(pedidas.borrow().is_empty(), "no natural voice was made");
        assert!(!p.join(TOMAS).exists() && !p.join("voz.wav").exists());
        let _ = std::fs::remove_dir_all(&d);
    }
}
