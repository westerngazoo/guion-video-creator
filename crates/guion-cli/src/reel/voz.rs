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
//! Nothing is written unless everything was synthesized and checked.

use std::path::{Path, PathBuf};

use guion_audio::{
    narrate_cast, Line, Narration, NarrationEngine, Pacing, PiperEngine, SilenceRule,
};

const SR: u32 = 48_000;

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
}

struct Guion {
    modelo: String,
    lento: f64,
    pacing: Pacing,
    voces: Vec<Reparto>,
    lineas: Vec<Linea>,
}

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
    let voz = narrate_cast(&lineas, &motores, &g.pacing, &SilenceRule::PIPER)
        .map_err(|e| e.to_string())?;
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
    Ok(())
}

/// The deterministic Piper for `modelo` (`<voices>/<modelo>/<modelo>.onnx`).
fn piper(voice: &Voice, modelo: &str, lento: f64) -> Result<PiperEngine, String> {
    let onnx = voice.voices.join(modelo).join(format!("{modelo}.onnx"));
    if !onnx.exists() {
        return Err(format!("no voice at {}", onnx.display()));
    }
    let mut p = PiperEngine::deterministic(onnx.display().to_string())
        .length_scale(lento)
        .sentence_silence(0.25);
    if let Some(b) = &voice.piper {
        p = p.binary(b.clone());
    }
    Ok(p)
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
        claves(t, &donde, &["modelo", "lento"])?;
        let modelo = t
            .get("modelo")
            .and_then(|m| m.as_str())
            .ok_or(format!("{donde}: falta modelo (el nombre de una voz)"))?
            .to_string();
        voces.push(Reparto {
            nombre: nombre.clone(),
            modelo,
            lento: numero(t, "lento", &donde)?.unwrap_or(lento),
        });
    }
    Ok(voces)
}

fn leer_guion(fuente: &str) -> Result<Guion, String> {
    let raiz: toml::Table = fuente.parse().map_err(|e: toml::de::Error| e.to_string())?;
    claves(&raiz, "", &["voz", "voces", "linea"])?;
    let v = raiz
        .get("voz")
        .and_then(|v| v.as_table())
        .ok_or("falta la tabla [voz]")?;
    claves(v, "[voz]", &["modelo", "inicio", "pausa", "lento"])?;
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
    let voces = leer_voces(&raiz, lento)?;
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
}
