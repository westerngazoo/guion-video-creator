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
//! lexico = ["guion:es", "../../lexico.toml"]   # optional: how terms are said
//!
//! [lexico]                    # optional: this post's own terms (they win)
//! "backprop" = "bácprop"
//!
//! [[linea]]                   # no blank line between two lines: one block,
//! id = "a"                    # spoken as one utterance
//! texto = "Primera frase."
//! [[linea]]
//! id = "b"
//! texto = "Segunda frase."
//!
//! [[linea]]                   # a blank line: a new block
//! id = "c"
//! texto = "Otra."
//! sigue = true                # …unless `sigue` says otherwise (true/false)
//! ```
//!
//! An account's `cuenta.toml` `[voz.prosodia]` replaces «C»'s numbers for its
//! natural posts; `voz.toml`'s own replace the account's. Each block is then
//! spoken once and kept in `voz.tomas/` (commit it: a take with noise cannot
//! be made again), and `voz.wav` and `voz.json` are derived from the takes,
//! so a re-run speaks nothing and gives the same bytes. Inside a block each
//! line but the last ends its sentence, and the pause between lines is the
//! voice's own (plus `pausa`/`antes`); `[voz] pausa` separates blocks. The
//! lexicon changes what the voice says, never what is written (`voz.json`).
//! Without `prosodia` the voice is the legacy one, byte for byte.
//!
//! **An external voice (R-0020).** `externa = "<file>"` in `[voz]` (with
//! `prosodia = "natural"`, without Piper's numbers) makes the main voice a
//! program ([`guion_audio::CommandVoice`]) described by that TOML file:
//! `programa`, `argumentos` (with `{salida}` and `{silencio}`), `silencio`,
//! `identidad`; its takes are kept like Piper's.
//!
//! Nothing is written unless everything was synthesized and checked.

use std::path::{Path, PathBuf};

use guion_audio::{
    compose, narrate_cast, narrate_utterances, split_sentences, CommandVoice, Lexicon, Line,
    Narration, NarrationEngine, Pacing, PiperEngine, Prosody, SilenceRule, TakeNarration,
    TakeStore, Utterance,
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
    /// It starts a block: an utterance of its own (R-0019 AC7). The first
    /// line always does.
    bloque: bool,
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
    /// `[voz] lexico`: the lists, in order (R-0019 AC11).
    lexicos: Vec<String>,
    /// `[lexico]` of `voz.toml` itself: it comes last.
    lexico: Lexicon,
    /// `[voz] externa`: the file describing an external voice (R-0020),
    /// relative to the post.
    externa: Option<String>,
}

/// The starter list's name in `[voz] lexico` (R-0019 AC11).
const LEXICO_GUION: &str = "guion:es";

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
    let principal = match &g.externa {
        Some(e) => Box::new(externa(&post.join(e))?) as Box<dyn guion_audio::Voice>,
        None => hacer(
            &g.modelo,
            prosodia(preset, cuenta, g.ajustes, Ajustes::default(), g.lento)?,
        )?,
    };
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
    let lexico = lexico(post, g)?;
    // one utterance per block (R-0019 AC7)
    let mut utterances: Vec<Utterance> = Vec::new();
    for (i, l) in g.lineas.iter().enumerate() {
        if l.bloque || utterances.is_empty() {
            utterances.push(Utterance {
                lines: i..i + 1,
                voice: match l.voz {
                    None => principal.as_ref(),
                    Some(k) => reparto[k].as_ref(),
                },
            });
        } else if let Some(u) = utterances.last_mut() {
            u.lines.end = i + 1;
        }
    }
    let lineas: Vec<Line> = g.lineas.iter().map(|l| l.linea.clone()).collect();
    // what the voice says: composed (AC10), then the lexicon (AC11)
    let dichas: Vec<String> = lineas
        .iter()
        .map(|l| lexico.apply(&compose(&l.text)))
        .collect();
    if g.externa.is_some() {
        frases_de_bloque(g, &utterances, &dichas)?;
    }
    let store = TakeStore::new(post.join(TOMAS));
    let r = narrate_utterances(&lineas, &dichas, &utterances, &g.pacing, &store)
        .map_err(|e| e.to_string())?;
    Ok((r, store))
}

/// The voice's lexicon (R-0019 AC11): the lists of `[voz] lexico` in order
/// (`"guion:es"`, the starter list, or a TOML file with a `[lexico]` table,
/// relative to the post), then `voz.toml`'s own `[lexico]`.
fn lexico(post: &Path, g: &Guion) -> Result<Lexicon, String> {
    let mut l = Lexicon::new();
    for fuente in &g.lexicos {
        if fuente == LEXICO_GUION {
            l.extend(&Lexicon::starter_es());
            continue;
        }
        if fuente.starts_with("guion:") {
            return Err(format!(
                "[voz] lexico: «{fuente}» no existe (la lista de la casa es «{LEXICO_GUION}»)"
            ));
        }
        let ruta = post.join(fuente);
        let texto =
            std::fs::read_to_string(&ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;
        let raiz: toml::Table = texto
            .parse()
            .map_err(|e: toml::de::Error| format!("{}: {e}", ruta.display()))?;
        claves(&raiz, "", &["lexico"]).map_err(|e| format!("{}: {e}", ruta.display()))?;
        let tabla = raiz
            .get("lexico")
            .and_then(|t| t.as_table())
            .ok_or(format!("{}: falta la tabla [lexico]", ruta.display()))?;
        l.extend(&leer_lexico(
            tabla,
            &format!("{} [lexico]", ruta.display()),
        )?);
    }
    l.extend(&g.lexico);
    Ok(l)
}

/// A `[lexico]` table: `"término" = "como lo dice la voz"`.
fn leer_lexico(t: &toml::Table, donde: &str) -> Result<Lexicon, String> {
    let mut l = Lexicon::new();
    for (k, v) in t {
        let v = v.as_str().ok_or(format!("{donde}: «{k}» no es texto"))?;
        l.insert(&compose(k), &compose(v))
            .map_err(|e| format!("{donde}: {e}"))?;
    }
    Ok(l)
}

/// For each `[[linea]]` after the first, whether a blank line comes between
/// it and the one before (R-0019 AC7: a blank line starts a block). Read
/// from the source, since TOML forgets its layout: a header is a line that
/// says `[[linea]]` (spaces inside the brackets and a comment after allowed);
/// a blank line inside a multi-line string does not count; any other table
/// between two lines counts as a blank line.
fn blancos_entre_lineas(fuente: &str) -> Vec<bool> {
    let mut out = Vec::new();
    let mut blanco = false;
    let mut cabeceras = 0;
    let mut cadena: Option<&str> = None;
    for renglon in fuente.lines() {
        if let Some(q) = cadena {
            if renglon.contains(q) {
                cadena = None;
            }
            continue;
        }
        let t = renglon.trim();
        for q in ["\"\"\"", "'''"] {
            if t.matches(q).count() % 2 == 1 {
                cadena = Some(q);
            }
        }
        let cabecera: String = t
            .split('#')
            .next()
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        if t.is_empty() {
            blanco = true;
        } else if cabecera == "[[linea]]" {
            if cabeceras > 0 {
                out.push(blanco);
            }
            cabeceras += 1;
            blanco = false;
        } else if t.starts_with('[') {
            blanco = true;
        }
    }
    out
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

/// The external voice described by the TOML file at `ruta` (R-0020):
///
/// ```toml
/// programa = "${QWEN_TTS_PYTHON}"      # ${VAR} comes from the environment
/// argumentos = ["-I", "${GUION_REPO}/tools/voz-qwen/voz_qwen.py",
///   "--modelo", "${QWEN_TTS_BASE}", "--referencia", "serena.wav",
///   "--texto-referencia", "serena.txt", "--semilla", "7",
///   "--silencio", "{silencio}", "{salida}"]
/// silencio = 0.3                       # s of zeros between two sentences
/// identidad = ["serena.wav", "serena.txt",   # files whose bytes decide the voice:
///   "${QWEN_TTS_BASE}/model.safetensors"]     # the reference and the weights (not the
///                                             # scripts: a fix to them keeps the takes)
/// ```
///
/// The program runs in the file's folder. The voice's identity is
/// `programa` and `argumentos` as written (never expanded: no machine's
/// paths) and the bytes of each `identidad` file. Fails on an unknown or a
/// missing key, a variable not set, or what [`CommandVoice::new`] refuses.
fn externa(ruta: &Path) -> Result<CommandVoice, String> {
    let donde = ruta.display().to_string();
    let fuente = std::fs::read_to_string(ruta).map_err(|e| format!("{donde}: {e}"))?;
    let t: toml::Table = fuente
        .parse()
        .map_err(|e: toml::de::Error| format!("{donde}: {e}"))?;
    claves(
        &t,
        &donde,
        &["programa", "argumentos", "silencio", "identidad"],
    )?;
    let texto = |k: &str| -> Result<String, String> {
        t.get(k)
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .ok_or(format!("{donde}: falta {k} (texto)"))
    };
    let lista = |k: &str| -> Result<Vec<String>, String> {
        t.get(k)
            .and_then(|x| x.as_array())
            .and_then(|a| a.iter().map(|s| s.as_str().map(str::to_string)).collect())
            .ok_or(format!("{donde}: falta {k} (una lista de textos)"))
    };
    let programa = texto("programa")?;
    let argumentos = lista("argumentos")?;
    let identidad = lista("identidad")?;
    if identidad.is_empty() {
        return Err(format!(
            "{donde}: identidad está vacía: nombra los archivos que deciden una toma \
             (la referencia, los scripts, los pesos), o una voz nueva reusaría tomas viejas"
        ));
    }
    let silencio = numero(&t, "silencio", &donde)?.ok_or(format!("{donde}: falta silencio"))?;
    let expandir = |s: &String| super::config::expand(s).map_err(|e| format!("{donde}: {e}"));
    let dir = ruta.parent().unwrap_or(Path::new(".")).to_path_buf();
    CommandVoice::new(
        expandir(&programa)?,
        argumentos.iter().map(expandir).collect::<Result<_, _>>()?,
        dir,
        silencio,
        format!("{programa} {}", argumentos.join(" ")),
        identidad
            .iter()
            .map(|s| expandir(s).map(PathBuf::from))
            .collect::<Result<_, _>>()?,
    )
    .map_err(|e| format!("{donde}: {e}"))
}

/// An external voice gets each block's sentences one per line, cut from the
/// block's text (R-0020 AC1); the take is then cut by each line's own count.
/// Both must agree before anything is spoken: «Dijo «sí.»» followed by
/// «Tres.» is two sentences line by line and one in the joined text.
fn frases_de_bloque(g: &Guion, utterances: &[Utterance], dichas: &[String]) -> Result<(), String> {
    for u in utterances {
        let lineas = &dichas[u.lines.clone()];
        let por_linea: usize = lineas.iter().map(|d| split_sentences(d).len()).sum();
        let juntas = split_sentences(&lineas.join(" ")).len();
        if por_linea != juntas {
            let ids: Vec<&str> = g.lineas[u.lines.clone()]
                .iter()
                .map(|l| l.linea.id.as_str())
                .collect();
            return Err(format!(
                "el bloque {} tiene {por_linea} frases línea por línea y {juntas} dicho de corrido \
                 (¿un cierre como .» seguido de otra línea?): cierra la frase fuera de las comillas \
                 o empieza un bloque nuevo",
                ids.join(", ")
            ));
        }
    }
    Ok(())
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
    claves(&raiz, "", &["voz", "voces", "linea", "lexico"])?;
    let v = raiz
        .get("voz")
        .and_then(|v| v.as_table())
        .ok_or("falta la tabla [voz]")?;
    claves(
        v,
        "[voz]",
        &[
            "modelo", "inicio", "pausa", "lento", "prosodia", "ruido", "ruido_w", "silencio",
            "lexico", "externa",
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
    let externa = match v.get("externa") {
        None => None,
        Some(e) => {
            let e = e
                .as_str()
                .ok_or("[voz]: externa es la ruta de un archivo (la voz externa, R-0020)")?;
            if prosodia != Some(Preset::Natural) {
                return Err(
                    "[voz]: externa pide prosodia = \"natural\" (sus tomas se guardan)".into(),
                );
            }
            if ajustes != Ajustes::default() || v.contains_key("lento") {
                return Err(
                    "[voz]: ruido, ruido_w, silencio y lento son de Piper; una voz externa \
                     trae su silencio en su archivo"
                        .into(),
                );
            }
            Some(e.to_string())
        }
    };
    let voces = leer_voces(&raiz, lento)?;
    for r in &voces {
        revisar_prosodia(
            prosodia,
            &r.ajustes,
            r.lento,
            &format!("[voces.{}]", r.nombre),
        )?;
    }
    // the pronunciation lexicon (R-0019 AC11): the natural voice only
    let lexicos = match v.get("lexico") {
        None => Vec::new(),
        Some(x) => x
            .as_array()
            .and_then(|a| a.iter().map(|s| s.as_str().map(str::to_string)).collect())
            .ok_or("[voz]: lexico es una lista de nombres (\"guion:es\" o un archivo)")?,
    };
    let lexico = match raiz.get("lexico") {
        None => Lexicon::new(),
        Some(t) => leer_lexico(t.as_table().ok_or("[lexico] no es una tabla")?, "[lexico]")?,
    };
    if prosodia.is_none() && (!lexicos.is_empty() || !lexico.is_empty()) {
        return Err(
            "el léxico ([voz] lexico, [lexico]) pide [voz] prosodia = \"natural\" o \
             \"determinista\" (sin prosodia, la voz es la de antes)"
                .into(),
        );
    }
    let tabla = raiz
        .get("linea")
        .and_then(|l| l.as_array())
        .ok_or("faltan las líneas ([[linea]])")?;
    // blocks: a blank line in the source, or `sigue` (R-0019 AC7)
    let blancos = blancos_entre_lineas(fuente);
    if prosodia.is_some() && blancos.len() + 1 != tabla.len() {
        return Err(format!(
            "no se pudo leer el acomodo de las {} líneas (se ven {} cabeceras [[linea]]): \
             marca los bloques con `sigue` en cada línea",
            tabla.len(),
            blancos.len() + 1
        ));
    }
    let mut lineas: Vec<Linea> = Vec::new();
    for (i, l) in tabla.iter().enumerate() {
        let donde = format!("[[linea]] {}", i + 1);
        let l = l.as_table().ok_or(format!("{donde}: no es una tabla"))?;
        claves(
            l,
            &donde,
            &[
                "id",
                "texto",
                "subtitulo",
                "pausa",
                "antes",
                "desde",
                "voz",
                "sigue",
            ],
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
        let sigue = match l.get("sigue") {
            None => None,
            Some(s) => Some(
                s.as_bool()
                    .ok_or(format!("{donde}: sigue es true o false"))?,
            ),
        };
        if prosodia.is_none() && sigue.is_some() {
            return Err(format!(
                "{donde}: sigue pide [voz] prosodia = \"natural\" o \"determinista\" (sin \
                 prosodia, cada línea se dice sola)"
            ));
        }
        let bloque = match lineas.last() {
            None => true,
            Some(antes) if antes.voz != voz => {
                if sigue == Some(true) {
                    return Err(format!(
                        "{donde}: sigue = true, pero la dice otra voz que la línea anterior: \
                         cada voz habla en su propio bloque"
                    ));
                }
                true
            }
            Some(_) => match sigue {
                Some(s) => !s,
                None => prosodia.is_none() || blancos[i - 1],
            },
        };
        let mut linea = Line::new(id, texto);
        linea.before = numero(l, "antes", &donde)?.unwrap_or(0.0);
        linea.after = numero(l, "pausa", &donde)?.unwrap_or(0.0);
        lineas.push(Linea {
            linea,
            subtitulo,
            desde,
            voz,
            bloque,
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
        lexicos,
        lexico,
        externa,
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

    use std::cell::RefCell;
    use std::rc::Rc;

    use guion_audio::{split_sentences, AudioError, Pcm16, TAKE_INDEX};

    /// Speaks like Piper with noise: 0.2 s per word at a level that changes
    /// on every call, and exact zeros between sentences; it keeps every text
    /// it was given. (A sibling of the fake in guion-audio's
    /// `tests/takes.rs`: a test crate cannot share it.)
    struct Ruidosa {
        llamadas: Rc<RefCell<Vec<String>>>,
        silencio: f64,
        id: String,
    }

    impl guion_audio::Voice for Ruidosa {
        fn identity(&self) -> Result<String, AudioError> {
            Ok(self.id.clone())
        }
        fn take(&self, text: &str) -> Result<Pcm16, AudioError> {
            self.llamadas.borrow_mut().push(text.to_string());
            let n = self.llamadas.borrow().len();
            let nivel = 6000 + 41 * n as i16;
            let mut out = Vec::new();
            let frases = split_sentences(text);
            for (i, f) in frases.iter().enumerate() {
                for _ in f.split_whitespace() {
                    out.extend((0..9600).map(|k| if (k / 8) % 2 == 0 { nivel } else { -nivel }));
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
        llamadas: &Rc<RefCell<Vec<String>>>,
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
        let llamadas = Rc::new(RefCell::new(Vec::new()));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        let hacer = fabrica(&llamadas, &pedidas);
        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.borrow().len(), 2);
        assert_eq!(pedidas.borrow()[0].1, Prosody::NATURAL);
        let wav = std::fs::read(p.join("voz.wav")).unwrap();
        let json = std::fs::read_to_string(p.join("voz.json")).unwrap();
        assert!(json.contains("\"subtitulo\": \"Uno.\""), "{json}");

        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(llamadas.borrow().len(), 2, "a re-run speaks nothing");
        assert_eq!(std::fs::read(p.join("voz.wav")).unwrap(), wav);
        assert_eq!(std::fs::read_to_string(p.join("voz.json")).unwrap(), json);

        let editado = NATURAL
            .replace("subtitulo = \"Uno.\"", "subtitulo = \"Tres.\"")
            .replace("antes = 0.2", "antes = 0.6");
        std::fs::write(p.join("voz.toml"), &editado).unwrap();
        narrate_con(&p, &v, &hacer).unwrap();
        assert_eq!(
            llamadas.borrow().len(),
            2,
            "subtitles and pauses are not takes"
        );

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
        assert_eq!(
            llamadas.borrow().len(),
            3,
            "only the edited line is spoken again"
        );
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

    /// A post whose `[voz] externa` names `voz/externa.toml`: `sh` counts its
    /// calls and copies a take with two sentences cut by 0.3 s of zeros.
    fn post_externo(nombre: &str, voz_extra: &str, archivo: &str) -> (PathBuf, Voice) {
        let toml = format!(
            "[voz]\nmodelo = \"serena\"\nprosodia = \"natural\"\nexterna = \"voz/externa.toml\"\n\
             {voz_extra}\n[[linea]]\nid = \"a\"\ntexto = \"Uno dos. Tres.\"\n"
        );
        let (d, v) = post(nombre, &toml);
        let voz = d.join("post/voz");
        std::fs::create_dir_all(&voz).unwrap();
        let mut take: Vec<i16> = (0..4800)
            .map(|k| if k % 2 == 0 { 6000 } else { -6000 })
            .collect();
        take.resize(4800 + 7200, 0);
        take.extend((0..4800).map(|k| if k % 2 == 0 { 5000 } else { -5000 }));
        guion_audio::write_wav_pcm16(&voz.join("ref.wav"), 24_000, &take).unwrap();
        std::fs::write(voz.join("externa.toml"), archivo).unwrap();
        (d, v)
    }

    const EXTERNA: &str = "programa = \"sh\"\n\
        argumentos = [\"-c\", \"cat > dicho.txt; echo x >> llamadas.txt; cp ref.wav \\\"$0\\\"\", \"{salida}\"]\n\
        silencio = 0.3\nidentidad = [\"ref.wav\"]\n";

    fn sin_piper() -> impl Fn(&str, Prosody) -> Result<Box<dyn guion_audio::Voice>, String> {
        |_: &str, _: Prosody| Err("una voz externa no llama a Piper".to_string())
    }

    /// R-0020 AC1, AC2: an external voice speaks the block through its
    /// program, sentence by sentence on stdin, once; a re-run reads the take.
    #[test]
    fn una_voz_externa_habla_una_vez_por_su_programa() {
        let (d, v) = post_externo("externa", "", EXTERNA);
        let p = d.join("post");
        narrate_con(&p, &v, &sin_piper()).unwrap();
        let voz = p.join("voz");
        assert_eq!(
            std::fs::read_to_string(voz.join("dicho.txt")).unwrap(),
            "Uno dos.\nTres.\n"
        );
        let json = std::fs::read_to_string(p.join("voz.json")).unwrap();
        assert!(json.contains("\"texto\": \"Tres.\""), "{json}");
        let wav = std::fs::read(p.join("voz.wav")).unwrap();
        narrate_con(&p, &v, &sin_piper()).unwrap();
        assert_eq!(
            std::fs::read_to_string(voz.join("llamadas.txt")).unwrap(),
            "x\n",
            "a re-run speaks nothing"
        );
        assert_eq!(std::fs::read(p.join("voz.wav")).unwrap(), wav);
        let indice = std::fs::read_to_string(p.join(TOMAS).join(TAKE_INDEX)).unwrap();
        assert!(indice.contains("voice: command sh -c"), "{indice}");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// R-0020 AC1: a block whose sentences cut differently line by line and
    /// said in one go fails before the voice speaks.
    #[test]
    fn un_bloque_que_corta_distinto_falla_antes_de_hablar() {
        let (d, v) = post_externo("externa-comillas", "", EXTERNA);
        let p = d.join("post");
        std::fs::write(
            p.join("voz.toml"),
            "[voz]\nmodelo = \"serena\"\nprosodia = \"natural\"\nexterna = \"voz/externa.toml\"\n\
             [[linea]]\nid = \"a\"\ntexto = \"Dijo «sí.»\"\n[[linea]]\nid = \"b\"\ntexto = \"Tres.\"\n",
        )
        .unwrap();
        let e = narrate_con(&p, &v, &sin_piper()).unwrap_err();
        assert!(e.contains("el bloque a, b"), "{e}");
        assert!(!p.join("voz/llamadas.txt").exists(), "nothing was spoken");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// R-0020 AC3: what an external voice cannot take fails by name.
    #[test]
    fn la_voz_externa_invalida_falla_por_nombre() {
        let casos = [
            ("lento = 0.9", EXTERNA.to_string(), "lento"),
            ("ruido = 0.5", EXTERNA.to_string(), "ruido"),
            (
                "",
                EXTERNA.replace("silencio = 0.3\n", ""),
                "falta silencio",
            ),
            ("", EXTERNA.replace("{salida}", "x.wav"), "{salida}"),
            (
                "",
                EXTERNA.replace("\"sh\"", "\"${GUION_R0020_NO_ESTA}\""),
                "GUION_R0020_NO_ESTA",
            ),
            ("", format!("{EXTERNA}voz = 1\n"), "voz"),
            (
                "",
                EXTERNA.replace("identidad = [\"ref.wav\"]", "identidad = []"),
                "identidad",
            ),
        ];
        for (i, (extra, archivo, espera)) in casos.iter().enumerate() {
            let (d, v) = post_externo(&format!("externa-mal-{i}"), extra, archivo);
            let e = narrate_con(&d.join("post"), &v, &sin_piper()).unwrap_err();
            assert!(e.contains(espera), "caso {i}: «{e}» no nombra «{espera}»");
            assert!(
                !d.join("post/voz.wav").exists(),
                "caso {i}: nothing is written"
            );
            let _ = std::fs::remove_dir_all(&d);
        }
        let sin = "[voz]\nmodelo = \"m\"\nexterna = \"x.toml\"\n[[linea]]\nid = \"a\"\ntexto = \"Hola.\"\n";
        let e = leer_guion(sin).err().unwrap();
        assert!(e.contains("prosodia"), "{e}");
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
        let llamadas = Rc::new(RefCell::new(Vec::new()));
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
        let llamadas = Rc::new(RefCell::new(Vec::new()));
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

    /// The blocks of a parsed `voz.toml`: the ids, grouped.
    fn bloques(g: &Guion) -> Vec<Vec<&str>> {
        let mut out: Vec<Vec<&str>> = Vec::new();
        for l in &g.lineas {
            if l.bloque {
                out.push(Vec::new());
            }
            out.last_mut().unwrap().push(&l.linea.id);
        }
        out
    }

    /// R-0019 AC7: lines with no blank line between them form a block; a
    /// blank line (or another table, but not a blank line inside a
    /// multi-line string) starts one; `sigue` decides over the layout; a
    /// change of voice always starts one.
    #[test]
    fn los_bloques_salen_del_renglon_en_blanco_y_de_sigue() {
        let g = leer_guion(concat!(
            "[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\n",
            "[voces.profe]\nmodelo = \"p\"\n\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Uno.\"\n",
            "# a comment is no blank line\n",
            "[[linea]]\nid = \"b\"\ntexto = \"\"\"Dos.\n\nTres.\"\"\"\n",
            "[[linea]]\nid = \"c\"\ntexto = \"Cuatro.\"\nsigue = false\n",
            "[[linea]]\nid = \"d\"\ntexto = \"Cinco.\"\n\n",
            "[[linea]]\nid = \"e\"\ntexto = \"Seis.\"\nsigue = true\n\n",
            "[[ linea ]]  # spaces inside the brackets\nid = \"f\"\ntexto = \"Siete.\"\n",
            "[[linea]]\nid = \"g\"\ntexto = \"Ocho.\"\nvoz = \"profe\"\n",
        ))
        .unwrap();
        assert_eq!(
            bloques(&g),
            [vec!["a", "b"], vec!["c", "d", "e"], vec!["f"], vec!["g"]]
        );
        // the legacy voice: one line, one utterance, whatever the layout
        let g = leer_guion(
            "[voz]\nmodelo = \"m\"\n[[linea]]\nid = \"a\"\ntexto = \"Uno.\"\n\
             [[linea]]\nid = \"b\"\ntexto = \"Dos.\"\n",
        )
        .unwrap();
        assert_eq!(bloques(&g), [vec!["a"], vec!["b"]]);
    }

    /// R-0019 AC2, AC7: the account template's `voz.toml`, filled as `guion
    /// reel nuevo` fills it, reads as a natural voice with the starter
    /// lexicon, one block per line.
    #[test]
    fn la_plantilla_de_la_casa_se_lee() {
        use guion_cuenta::nueva::{plan, Respuestas};
        let mut r = Respuestas::con_defaults("Cuenta", "tema", "revisor", "materia");
        r.fecha = "2026-11-26".into();
        let voz = plan(&r, "")
            .unwrap()
            .archivos
            .into_iter()
            .find(|a| a.ruta.ends_with("plantillas/reel/voz.toml"))
            .unwrap()
            .texto
            .replace("{{titulo}}", "¿Qué es un bit?")
            .replace("{{fecha}}", "2026-11-26")
            .replace("{{slug}}", "que-es-un-bit");
        let g = leer_guion(&voz).unwrap();
        assert_eq!(g.prosodia, Some(Preset::Natural));
        assert_eq!(g.lexicos, [LEXICO_GUION]);
        assert_eq!(bloques(&g), [vec!["gancho"], vec!["cierre"]]);
    }

    /// R-0019 AC7: `sigue` across voices, or without `prosodia`, fails by line.
    #[test]
    fn sigue_invalido_falla_por_linea() {
        let e = leer_guion(concat!(
            "[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\n[voces.profe]\nmodelo = \"p\"\n",
            "[[linea]]\nid = \"a\"\ntexto = \"Uno.\"\n",
            "[[linea]]\nid = \"b\"\ntexto = \"Dos.\"\nvoz = \"profe\"\nsigue = true\n",
        ))
        .err()
        .unwrap();
        assert!(e.contains("«b»") && e.contains("otra voz"), "{e}");
        let e = leer_guion(
            "[voz]\nmodelo = \"m\"\n[[linea]]\nid = \"a\"\ntexto = \"Uno.\"\nsigue = true\n",
        )
        .err()
        .unwrap();
        assert!(e.contains("«a»") && e.contains("prosodia"), "{e}");
    }

    /// R-0019 AC7, AC8: a block is one take; its lines keep their own times,
    /// joined by the voice's pause (no global `pausa` inside the block); a
    /// line inside a block that does not end its sentence fails, naming it,
    /// before anything is spoken.
    #[test]
    fn un_bloque_se_dice_de_un_tiron() {
        let toml = "[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\ninicio = 0.5\npausa = 0.3\n\n\
            [[linea]]\nid = \"a\"\ntexto = \"Uno dos.\"\n\
            [[linea]]\nid = \"b\"\ntexto = \"Tres.\"\n\n\
            [[linea]]\nid = \"c\"\ntexto = \"Cuatro.\"\n";
        let (d, v) = post("bloque", toml);
        let p = d.join("post");
        let llamadas = Rc::new(RefCell::new(Vec::new()));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap();
        assert_eq!(*llamadas.borrow(), ["Uno dos. Tres.", "Cuatro."]);
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(p.join("voz.json")).unwrap()).unwrap();
        let t = |i: usize, k: &str| json["lineas"][i][k].as_f64().unwrap();
        // a: 0.5 → 0.9; the voice's 0.35 s; b: 1.25 → 1.45; the global 0.3; c: 1.75
        assert_eq!((t(0, "t0"), t(0, "t1")), (0.5, 0.9));
        assert_eq!((t(1, "t0"), t(1, "t1")), (1.25, 1.45));
        assert_eq!(t(2, "t0"), 1.75);

        std::fs::write(
            p.join("voz.toml"),
            toml.replace("texto = \"Uno dos.\"", "texto = \"Uno dos:\""),
        )
        .unwrap();
        let antes = llamadas.borrow().len();
        let e = narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap_err();
        assert!(e.contains("«a»") && e.contains("media frase"), "{e}");
        assert_eq!(llamadas.borrow().len(), antes, "nothing spoken");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// R-0019 AC10, AC11: the voice says the text composed and through the
    /// lexicon (the starter list, a file, `[lexico]`, a later one winning);
    /// `voz.json` keeps what is written.
    #[test]
    fn el_lexico_cambia_lo_que_dice_la_voz_no_lo_escrito() {
        let toml = "[voz]\nmodelo = \"m\"\nprosodia = \"natural\"\n\
            lexico = [\"guion:es\", \"../lexico.toml\"]\n\n\
            [lexico]\n\"backprop\" = \"bacprop\"\n\n\
            [[linea]]\nid = \"a\"\ntexto = \"Cincuenta N·m con IA y backprop en el pu\u{301}blico.\"\n";
        let (d, v) = post("lexico", toml);
        std::fs::write(
            d.join("lexico.toml"),
            "[lexico]\n\"IA\" = \"inteligencia artificial\"\n\"backprop\" = \"NO\"\n",
        )
        .unwrap();
        let p = d.join("post");
        let llamadas = Rc::new(RefCell::new(Vec::new()));
        let pedidas = Rc::new(RefCell::new(Vec::new()));
        narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap();
        assert_eq!(
            llamadas.borrow()[0],
            "Cincuenta newton metro con inteligencia artificial y bacprop en el público."
        );
        let json = std::fs::read_to_string(p.join("voz.json")).unwrap();
        assert!(json.contains("Cincuenta N·m con IA y backprop"), "{json}");

        // a list that cannot be read, or an entry that moves a sentence cut, fail by name
        std::fs::write(d.join("lexico.toml"), "[lexico]\n\"etc.\" = \"etcétera\"\n").unwrap();
        let e = narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap_err();
        assert!(e.contains("lexico.toml") && e.contains("etc."), "{e}");
        std::fs::write(
            p.join("voz.toml"),
            toml.replace("\"guion:es\"", "\"guion:en\""),
        )
        .unwrap();
        let e = narrate_con(&p, &v, &fabrica(&llamadas, &pedidas)).unwrap_err();
        assert!(e.contains("guion:en") && e.contains("guion:es"), "{e}");
        // the legacy voice takes no lexicon
        let e = leer_guion(
            "[voz]\nmodelo = \"m\"\nlexico = [\"guion:es\"]\n[[linea]]\nid = \"a\"\ntexto = \"x\"\n",
        )
        .err()
        .unwrap();
        assert!(e.contains("léxico") && e.contains("prosodia"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
