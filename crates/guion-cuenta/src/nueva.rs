//! `guion cuenta nueva`: a new account, scaffolded from the generic templates
//! (R-0014 AC3–AC5).
//!
//! The answers ([`Respuestas`]) come from flags or a prompt; [`plan`] turns
//! them into every file the account starts with, as text, in a fixed order;
//! [`crear`] writes them, and refuses before writing anything if one of them
//! already exists. Nothing depends on the clock or the machine except what
//! the answers say (the date is an answer), so the same answers give the same
//! bytes.

use std::path::{Component, Path, PathBuf};

use crate::contrato::{fecha_valida, slug_valido, Cuenta, IDIOMAS};
use crate::perfil;
use crate::plantillas;
use crate::texto::{json_str, toml_lista, toml_str};

/// The answers a new account is made from.
#[derive(Debug, Clone, PartialEq)]
pub struct Respuestas {
    pub nombre: String,
    pub slug: String,
    /// Empty: not reserved yet.
    pub handle: String,
    pub idioma: String,
    pub tema: String,
    pub publico: String,
    pub dueno: String,
    /// The day it is created, `AAAA-MM-DD` (the first row of decisions.md).
    pub fecha: String,
    pub voz_modelo: String,
    pub tono: String,
    pub trato: String,
    pub revisor: String,
    pub materia: String,
    pub reglas: Vec<String>,
    /// Tokens that replace or join the neutral palette.
    pub paleta: Vec<(String, [u8; 3])>,
    pub fuente_titulo: String,
    pub fuente_texto: String,
    pub hashtags: Vec<String>,
    /// guion-video-creator, as seen from the account's root (or absolute).
    /// `None`: the sibling of the repo the account lives in.
    pub guion: Option<String>,
}

impl Respuestas {
    /// The defaults for what was not answered: everything but the name, the
    /// subject and the reviewer.
    pub fn con_defaults(nombre: &str, tema: &str, revisor: &str, materia: &str) -> Respuestas {
        Respuestas {
            nombre: nombre.to_string(),
            slug: crate::texto::a_slug(nombre),
            handle: String::new(),
            idioma: "es".into(),
            tema: tema.to_string(),
            publico: String::new(),
            dueno: String::new(),
            fecha: String::new(),
            voz_modelo: String::new(),
            tono: String::new(),
            trato: String::new(),
            revisor: revisor.to_string(),
            materia: materia.to_string(),
            reglas: Vec::new(),
            paleta: Vec::new(),
            fuente_titulo: "brand/fonts/titulo.ttf".into(),
            fuente_texto: "brand/fonts/texto.ttf".into(),
            hashtags: Vec::new(),
            guion: None,
        }
    }
}

/// The neutral palette a new account starts with (its design direction is
/// decided by hand: R-0014 §4).
pub const PALETA_NEUTRA: [(&str, [u8; 3]); 6] = [
    ("fondo", [12, 12, 14]),
    ("panel", [24, 24, 28]),
    ("tinta", [236, 238, 242]),
    ("tenue", [128, 130, 138]),
    ("acento", [64, 140, 255]),
    ("foco", [240, 190, 70]),
];

/// What the gate never lets through in a caption, whatever the account.
const INTERNO: [&str; 15] = [
    "R-####",
    "OQ-#",
    "guion-video-creator",
    "guion-reel",
    "guion-audio",
    "guion-publish",
    "guion-marca",
    "motoreel",
    "garust",
    "cuenta.toml",
    "reel.toml",
    "voz.toml",
    "sonido.toml",
    "escena.rs",
    "eventos.json",
];

/// Units that take U+202F after a number.
const UNIDADES: [&str; 19] = [
    "%", "km/h", "m/s", "km", "kg", "GHz", "MHz", "Hz", "TB", "GB", "MB", "kB", "ms", "ns", "nm",
    "mm", "cm", "W", "m",
];

/// The strings a template needs in the published language.
struct Lengua {
    voz: &'static str,
    trato: &'static str,
    prohibido: &'static [&'static str],
    gancho_sub: &'static str,
    cierre_texto: &'static str,
    cierre_sub: &'static str,
    caption: &'static str,
    alt: &'static str,
    hashtags: &'static str,
}

fn lengua(idioma: &str) -> Lengua {
    match idioma {
        "en" => Lengua {
            voz: "en-gb-alan-low",
            trato: "you, plain and direct",
            prohibido: &["incredible", "mind-blowing"],
            gancho_sub: "The question, in short.",
            cierre_texto: "The sentence that closes the idea.",
            cierre_sub: "The sentence that closes.",
            caption: "The answer in one or two sentences, the intuition before the formula. Every number says where it comes from: an example, calculated (not measured), measured (when) or reported (the source is in the README).",
            alt: "A narrated animation with subtitles. What is seen, beat by beat, in the order it appears. It closes on the account's card.",
            hashtags: "#science #technology",
        },
        _ => Lengua {
            voz: "es-carlfm-x-low",
            trato: "tuteo",
            prohibido: &["increíble"],
            gancho_sub: "La pregunta, en corto.",
            cierre_texto: "La frase que cierra la idea.",
            cierre_sub: "La frase que cierra.",
            caption: "La respuesta en una o dos frases, con la intuición antes que la fórmula. Cada número dice su procedencia: de ejemplo, calculado (no medido), medido (cuándo) o reportado (la fuente va en el README).",
            alt: "Animación narrada, con subtítulos. Lo que se ve, beat por beat, en el orden en que aparece. Cierra la tarjeta con el nombre de la cuenta.",
            hashtags: "#ciencia #tecnología",
        },
    }
}

/// One file of the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Archivo {
    /// Relative to the repo's root (the account's own folder when it is its
    /// own repo).
    pub ruta: String,
    pub texto: String,
}

/// Everything a new account is.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The account's folder, relative to the repo's root (`""`: the root).
    pub prefijo: String,
    pub archivos: Vec<Archivo>,
    /// The `[[cuenta]]` entry of `guion-cuentas.toml` (paths from the root).
    pub entrada: String,
    /// The contract, as written.
    pub cuenta: Cuenta,
}

fn hex(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

/// `fondo=#0a0a0b,acento=#e0322a` → tokens.
pub fn parse_paleta(s: &str) -> Result<Vec<(String, [u8; 3])>, String> {
    s.split(',')
        .filter(|x| !x.trim().is_empty())
        .map(|kv| {
            let (k, v) = kv
                .split_once('=')
                .ok_or(format!("--paleta «{kv}»: nombre=#rrggbb"))?;
            let k = k.trim();
            if !slug_valido(k) {
                return Err(format!("--paleta «{k}»: a token is a slug"));
            }
            Ok((
                k.to_string(),
                hex(v.trim()).ok_or(format!("--paleta «{kv}»: #rrggbb"))?,
            ))
        })
        .collect()
}

fn arriba(n: usize) -> String {
    vec![".."; n].join("/")
}

fn unir(a: &str, b: &str) -> String {
    match (a, b) {
        ("", b) => b.to_string(),
        (a, "") => a.to_string(),
        (a, b) => format!("{a}/{b}"),
    }
}

fn profundidad(prefijo: &str) -> usize {
    prefijo.split('/').filter(|s| !s.is_empty()).count()
}

/// Checks the answers; each failure names the flag.
pub fn validar(r: &Respuestas) -> Result<(), String> {
    if r.nombre.trim().is_empty() {
        return Err("--nombre is missing".into());
    }
    if !slug_valido(&r.slug) {
        return Err(format!(
            "--slug «{}»: lowercase letters, digits and hyphens, starting with a letter",
            r.slug
        ));
    }
    if !IDIOMAS.contains(&r.idioma.as_str()) {
        return Err(format!("--idioma «{}»: es or en", r.idioma));
    }
    if !fecha_valida(&r.fecha) {
        return Err(format!("--fecha «{}»: AAAA-MM-DD", r.fecha));
    }
    if r.tema.trim().is_empty() {
        return Err("--tema is missing".into());
    }
    if !slug_valido(&r.revisor) || r.revisor == "designer" {
        return Err(format!(
            "--revisor «{}»: a slug, not «designer» (it names the agent file)",
            r.revisor
        ));
    }
    if r.materia.trim().is_empty() {
        return Err("--materia is missing".into());
    }
    if !r.handle.is_empty() && !r.handle.starts_with('@') {
        return Err(format!("--handle «{}» starts with @", r.handle));
    }
    for t in &r.hashtags {
        if !t.starts_with('#') || t.len() < 2 {
            return Err(format!("--hashtags «{t}»: each starts with #"));
        }
    }
    Ok(())
}

/// The contract's text.
fn cuenta_toml(r: &Respuestas, l: &Lengua) -> String {
    let mut tokens: Vec<(String, [u8; 3])> = PALETA_NEUTRA
        .iter()
        .map(|(k, v)| (k.to_string(), *v))
        .collect();
    for (k, v) in &r.paleta {
        match tokens.iter_mut().find(|(n, _)| n == k) {
            Some(t) => t.1 = *v,
            None => tokens.push((k.clone(), *v)),
        }
    }
    let tokens_txt: String = tokens
        .iter()
        .map(|(k, v)| format!("{k} = [{}, {}, {}]\n", v[0], v[1], v[2]))
        .collect();
    let s = |x: &str| toml_str(x);
    let lista = |v: &[&str]| toml_lista(&v.iter().map(|x| x.to_string()).collect::<Vec<_>>());
    let voz = if r.voz_modelo.is_empty() {
        l.voz
    } else {
        &r.voz_modelo
    };
    let trato = if r.trato.is_empty() {
        l.trato
    } else {
        &r.trato
    };
    let tono = if r.tono.is_empty() {
        "por decidir con el dueño"
    } else {
        &r.tono
    };
    format!(
        r#"# El contrato de la cuenta (guion-video-creator R-0014): lo que la cuenta pone.
# La capa compartida (la compuerta, la voz, el sonido, el tablero, el proceso y
# los revisores genéricos) lo lee y no guarda ningún valor propio. Creado con
# `guion cuenta nueva` el {fecha}. Si cambias [marca] o [compuerta], corre
# `guion cuenta perfil .` para derivar brand/reel-perfil.toml otra vez.

[cuenta]
nombre = {nombre}
slug = {slug}
handle = {handle}                       # vacío: por reservar
idioma = {idioma}                       # lo que se publica: es | en
tema = {tema}
publico = {publico}
dueno = {dueno}
creada = {fecha_s}

[voz]
modelo = {voz}                          # el modelo de Piper (voz.toml lo nombra)
tono = {tono}
trato = {trato}
prohibido = {prohibido}                 # la compuerta los rechaza en el caption

[marca]
direccion = "por decidir"               # la dirección de diseño la decide el dueño (R-0014 §4)
logo = ""                               # por decidir; la fase 2 lo dibuja desde aquí (R-0014 §5)
lienzo = {{ ancho = 1080, alto = 1920, fps = 30.0 }}

# La paleta: neutra hasta que se decida la dirección de diseño.
[marca.tokens]
{tokens}
# El papel de cada token (la compuerta los cuenta: un acento y un foco por cuadro).
[marca.roles]
fondo = ["fondo", "panel"]
texto = "tinta"
secundario = "tenue"
acento = ["acento"]
foco = ["foco"]

# Las caras, relativas a la raíz de la cuenta (no se versionan: brand/fonts/README.md).
[marca.fuentes]
titulo = {ft}
texto = {fx}

[marca.titulo]
tam = 88.0                              # px de em
y = 760.0                               # línea base

# El subtítulo: un renglón de 840 px o menos centrado, línea base 1500, la tinta
# antes de 1540; solo palabras, 8 o menos; más de 6 s → `desde`; fundido de 0.15 s.
[marca.subtitulo]
tam = 52.0
ancho = 840.0
base = 1500.0
franja = [1460.0, 1545.0]
palabras = 8
fundido = 0.15
linea_larga = 6.0
prohibidos = "=√≤≥±×·²³₀₁₂%+−_^"
sin_cifras = true

# Libres de texto los 250 px de arriba, los 380 de abajo y 90 a cada lado (los
# botones de Instagram). `excepto`: rectángulos exentos (una marca de agua).
[marca.seguro]
x = [90.0, 990.0]
y = [250.0, 1540.0]
excepto = []

# Los límites de la compuerta que no son dibujo: movimiento, audio y caption.
[compuerta]
quieto_con_voz = 4.0                    # ningún cuadro quieto de más de 4 s con voz
cierre_quieto = 1.5                     # la tarjeta de cierre se queda 1.5 s o más
lufs = -14.0                            # −14 LUFS ± 0.5, pico real −1 dBTP o menos
tolerancia_lufs = 0.5
pico = -1.0
hueco_corto = 0.5                       # la cama no sube en huecos de menos de 0.5 s
hueco_max_db = -15.0
eventos_de_beat = ["beat-"]
max_hashtags = 5                        # el tope de Instagram desde el 2025-12-19
max_caracteres = 2200
unidades = {unidades}
interno = {interno}

# El revisor de la materia: va antes que el diseño; un FIX suyo detiene el post.
[revisor]
nombre = {revisor}
materia = {materia}
reglas = [
{reglas}]
"#,
        fecha = r.fecha,
        nombre = s(&r.nombre),
        slug = s(&r.slug),
        handle = s(&r.handle),
        idioma = s(&r.idioma),
        tema = s(&r.tema),
        publico = s(&r.publico),
        dueno = s(&r.dueno),
        fecha_s = s(&r.fecha),
        voz = s(voz),
        tono = s(tono),
        trato = s(trato),
        prohibido = lista(l.prohibido),
        tokens = tokens_txt,
        ft = s(&r.fuente_titulo),
        fx = s(&r.fuente_texto),
        unidades = lista(&UNIDADES),
        interno = lista(&INTERNO),
        revisor = s(&r.revisor),
        materia = s(&r.materia),
        reglas = r
            .reglas
            .iter()
            .map(|x| format!("  {},\n", s(x)))
            .collect::<String>(),
    )
}

/// Every file of the new account, and its accounts-file entry. `prefijo` is
/// the account's folder relative to the repo's root (`""` when the account is
/// its own repo).
pub fn plan(r: &Respuestas, prefijo: &str) -> Result<Plan, String> {
    validar(r)?;
    let l = lengua(&r.idioma);
    let texto_cuenta = cuenta_toml(r, &l);
    let cuenta = Cuenta::parse(&texto_cuenta)
        .map_err(|e| format!("the generated cuenta.toml does not hold: {e}"))?;
    let propio = prefijo.is_empty();
    let depth = profundidad(prefijo);
    // guion-video-creator from the account's root: the sibling of the repo
    let guion = match &r.guion {
        Some(g) => g.trim_end_matches('/').to_string(),
        None => unir(&arriba(depth + 1), "guion-video-creator"),
    };
    let guion_motor = if Path::new(&guion).is_absolute() {
        guion.clone()
    } else {
        format!("../{guion}")
    };
    // the house keeps one publicar/ at the repo's root
    let publicar_raiz = format!("publicar/{}", r.slug);
    let publicar_cuenta = unir(&arriba(depth), &publicar_raiz);
    let designer = if propio {
        "designer".to_string()
    } else {
        format!("designer-{}", r.slug)
    };
    let skill = format!("reel-{}", r.slug);
    let reglas_md = if r.reglas.is_empty() {
        "- (por decidir con el dueño: las reglas propias de la materia, una frase cada una)\n"
            .to_string()
    } else {
        r.reglas.iter().map(|x| format!("- {x}\n")).collect()
    };
    let m = &cuenta.marca;
    let tokens_md: String = m
        .tokens
        .iter()
        .map(|(k, v)| format!("| `{k}` | #{:02x}{:02x}{:02x} |\n", v[0], v[1], v[2]))
        .collect();
    let hashtags = if r.hashtags.is_empty() {
        l.hashtags.to_string()
    } else {
        r.hashtags.join(" ")
    };
    let handle_o_pendiente = if r.handle.is_empty() {
        "por reservar".to_string()
    } else {
        r.handle.clone()
    };
    let g = guion.clone();
    let valores: Vec<(&str, String)> = vec![
        ("nombre", r.nombre.clone()),
        ("slug", r.slug.clone()),
        ("handle", handle_o_pendiente),
        ("idioma", r.idioma.clone()),
        ("tema", r.tema.clone()),
        (
            "publico",
            if r.publico.is_empty() {
                "por decidir con el dueño".into()
            } else {
                r.publico.clone()
            },
        ),
        (
            "dueno",
            if r.dueno.is_empty() {
                "el dueño".into()
            } else {
                r.dueno.clone()
            },
        ),
        ("fecha", r.fecha.clone()),
        ("voz_modelo", cuenta.voz.modelo.clone()),
        ("tono", cuenta.voz.tono.clone()),
        ("trato", cuenta.voz.trato.clone()),
        ("revisor", r.revisor.clone()),
        ("materia", r.materia.clone()),
        ("reglas", reglas_md),
        ("designer", designer.clone()),
        ("skill", skill.clone()),
        ("guion", g.clone()),
        ("guion_motor", guion_motor),
        ("proceso", format!("{g}/docs/proceso-reel.md")),
        ("publicar", publicar_cuenta.clone()),
        ("publicar_post", format!("../../{publicar_cuenta}")),
        ("tokens_md", tokens_md),
        ("fuente_titulo", m.fuentes.titulo.clone()),
        ("fuente_texto", m.fuentes.texto.clone()),
        ("rol_fondo", m.roles.fondo.join(", ")),
        ("rol_texto", m.roles.texto.clone()),
        ("rol_acento", m.roles.acento.join(", ")),
        ("rol_foco", m.roles.foco.join(", ")),
        ("gancho_sub", l.gancho_sub.into()),
        ("cierre_texto", l.cierre_texto.into()),
        ("cierre_sub", l.cierre_sub.into()),
        ("caption", l.caption.into()),
        ("alt", l.alt.into()),
        ("hashtags", hashtags),
        ("prefijo", if propio { ".".into() } else { prefijo.into() }),
    ];
    let en = |ruta: &str| unir(prefijo, ruta);
    let mut archivos = vec![
        Archivo {
            ruta: en("cuenta.toml"),
            texto: texto_cuenta.clone(),
        },
        Archivo {
            ruta: en(perfil::RUTA),
            texto: perfil::perfil(&cuenta),
        },
        Archivo {
            ruta: en("estado.json"),
            texto: estado_json(&cuenta),
        },
        Archivo {
            ruta: publicar_raiz.clone() + "/perfil/README.md",
            texto: String::new(),
        },
    ];
    for (destino, plantilla) in plantillas::TODAS {
        let ruta = match *destino {
            "agente-revisor" => format!(".claude/agents/{}.md", r.revisor),
            "agente-designer" => format!(".claude/agents/{designer}.md"),
            "skill" => format!(".claude/skills/{skill}/SKILL.md"),
            "publicar-perfil" => {
                archivos[3].texto = plantillas::render(plantilla, &valores, destino)?;
                continue;
            }
            otro => en(otro),
        };
        // the agents and the skill live where Claude Code loads them: the repo's root
        archivos.push(Archivo {
            ruta,
            texto: plantillas::render(plantilla, &valores, destino)?,
        });
    }
    let entrada = entrada_cuentas(r, prefijo, &publicar_raiz);
    if propio {
        archivos.push(Archivo {
            ruta: "guion-cuentas.toml".into(),
            texto: format!(
                "# Las cuentas de este repo, para `guion tablero` y `guion reel nuevo`\n\
# (guion-video-creator R-0013). Las rutas son relativas a la raíz del repo.\n\
# Creado con `guion cuenta nueva` (R-0014).\n\n{entrada}"
            ),
        });
    }
    archivos.sort_by(|a, b| a.ruta.cmp(&b.ruta));
    Ok(Plan {
        prefijo: prefijo.to_string(),
        archivos,
        entrada,
        cuenta,
    })
}

/// The account's `[[cuenta]]` entry (R-0013 AC1), paths from the repo's root.
fn entrada_cuentas(r: &Respuestas, prefijo: &str, publicar: &str) -> String {
    let en = |x: &str| toml_str(&unir(prefijo, x));
    let bin = format!(
        "\n# {{{{titulo}}}} ({nombre}, creado con `guion reel nuevo` el {{{{fecha}}}})\n\
[[bin]]\nname = \"{{{{slug}}}}\"\npath = \"../posts/{{{{slug}}}}/escena.rs\"\n",
        nombre = r.nombre.replace('"', "'")
    );
    format!(
        "[[cuenta]]\nnombre = {nombre}\nposts = [{posts}]\nperfil = {perfil}\npublicar = [{publicar}]\n\
plantilla = {plantilla}\nnuevos = {nuevos}\n\n\
# La escena del post nuevo, registrada en el motor de la cuenta.\n\
[[cuenta.registro]]\narchivo = {motor}\ntexto = \"\"\"\n{bin}\"\"\"\n",
        nombre = toml_str(&r.nombre),
        posts = en("posts/*"),
        perfil = en(perfil::RUTA),
        publicar = toml_str(publicar),
        plantilla = en("plantillas/reel"),
        nuevos = en("posts"),
        motor = en("motor/Cargo.toml"),
    )
}

/// The status file: what the dashboard cannot read from a README (when a
/// piece went up, its reach). Starts empty.
fn estado_json(c: &Cuenta) -> String {
    format!(
        "{{\n \"_leeme\": [\n  {},\n  {},\n  {}\n ],\n \"cuenta\": {},\n \"handle\": {},\n \"creada\": {},\n \"piezas\": []\n}}\n",
        json_str("El estado de cada pieza publicada: la fuente de verdad del estado de un post es la línea `estado:` de su README.md (guion tablero la lee); aquí va lo que no cabe ahí."),
        json_str("pieza: {\"post\": \"<slug>\", \"estado\": \"listo|publicado|guardado\", \"fecha\": \"AAAA-MM-DD\", \"enlace\": null, \"vistas\": null, \"likes\": null}"),
        json_str("vistas / likes: null mientras no se sepan. No se inventan."),
        json_str(&c.cuenta.nombre),
        json_str(&c.cuenta.handle),
        json_str(&c.cuenta.creada),
    )
}

/// `path` made absolute against the current folder and cleaned of `.` and
/// `..` without touching the disk.
fn absoluta(p: &Path) -> Result<PathBuf, String> {
    let base = if p.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().map_err(|e| e.to_string())?
    };
    let mut out = PathBuf::new();
    for c in base.join(p).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            o => out.push(o),
        }
    }
    Ok(out)
}

/// The deepest existing ancestor canonicalized, the rest appended (so a
/// symlinked temp dir compares equal to its target).
fn canonica(p: &Path) -> Result<PathBuf, String> {
    let abs = absoluta(p)?;
    let mut existe = abs.clone();
    let mut resto = Vec::new();
    while !existe.exists() {
        match (existe.file_name(), existe.parent()) {
            (Some(n), Some(par)) => {
                resto.push(n.to_os_string());
                existe = par.to_path_buf();
            }
            _ => break,
        }
    }
    let mut out = existe.canonicalize().unwrap_or(existe);
    for n in resto.into_iter().rev() {
        out.push(n);
    }
    Ok(out)
}

/// What [`crear`] wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct Creada {
    /// The repo's root.
    pub raiz: PathBuf,
    /// The account's folder.
    pub dir: PathBuf,
    /// Files written, relative to the root, sorted.
    pub archivos: Vec<String>,
    /// The accounts file the entry went into.
    pub cuentas: PathBuf,
}

/// Writes a new account into `dir`. With `repo`, `dir` lies inside that repo
/// and its entry is appended to the repo's `guion-cuentas.toml`; without it,
/// `dir` is the account's own repo. Refuses, before writing anything, when a
/// file it would write exists, or when the repo already declares an account
/// with the same name.
pub fn crear(r: &Respuestas, dir: &Path, repo: Option<&Path>) -> Result<Creada, String> {
    let dir_c = canonica(dir)?;
    let (raiz, prefijo) = match repo {
        None => (dir_c.clone(), String::new()),
        Some(root) => {
            let root_c = canonica(root)?;
            if !root_c.join(guion_cuentas()).is_file() {
                return Err(format!(
                    "{}: --repo needs its guion-cuentas.toml",
                    root_c.join(guion_cuentas()).display()
                ));
            }
            let rel = dir_c.strip_prefix(&root_c).map_err(|_| {
                format!(
                    "{} is not inside the repo {}",
                    dir_c.display(),
                    root_c.display()
                )
            })?;
            let prefijo = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            if prefijo.is_empty() {
                return Err("with --repo, the account goes in a folder of the repo".into());
            }
            (root_c, prefijo)
        }
    };
    let p = plan(r, &prefijo)?;
    let cuentas = raiz.join(guion_cuentas());
    let mut agregado = None;
    if repo.is_some() {
        let actual =
            std::fs::read_to_string(&cuentas).map_err(|e| format!("{}: {e}", cuentas.display()))?;
        let nombres =
            nombres_declarados(&actual).map_err(|e| format!("{}: {e}", cuentas.display()))?;
        if nombres
            .iter()
            .any(|n| n.trim().to_lowercase() == r.nombre.trim().to_lowercase())
        {
            return Err(format!(
                "{}: the account «{}» is already declared (nothing was written)",
                cuentas.display(),
                r.nombre
            ));
        }
        let mut nuevo = actual.clone();
        if !nuevo.ends_with('\n') {
            nuevo.push('\n');
        }
        nuevo.push('\n');
        nuevo.push_str(&p.entrada);
        nombres_declarados(&nuevo)
            .map_err(|e| format!("the appended entry does not parse: {e}"))?;
        agregado = Some((actual, nuevo));
    }
    let existentes: Vec<&str> = p
        .archivos
        .iter()
        .filter(|a| raiz.join(&a.ruta).exists())
        .map(|a| a.ruta.as_str())
        .collect();
    if !existentes.is_empty() {
        return Err(format!(
            "refusing to overwrite {} (nothing was written): {}",
            raiz.display(),
            existentes.join(", ")
        ));
    }
    let mut escritos: Vec<PathBuf> = Vec::new();
    let r_escribir = (|| -> Result<(), String> {
        for a in &p.archivos {
            let f = raiz.join(&a.ruta);
            if let Some(par) = f.parent() {
                std::fs::create_dir_all(par).map_err(|e| format!("{}: {e}", par.display()))?;
            }
            std::fs::write(&f, &a.texto).map_err(|e| format!("{}: {e}", f.display()))?;
            escritos.push(f);
        }
        if let Some((_, nuevo)) = &agregado {
            std::fs::write(&cuentas, nuevo).map_err(|e| format!("{}: {e}", cuentas.display()))?;
        }
        Ok(())
    })();
    if let Err(e) = r_escribir {
        for f in escritos.iter().rev() {
            let _ = std::fs::remove_file(f);
        }
        if let Some((antes, _)) = &agregado {
            let _ = std::fs::write(&cuentas, antes);
        }
        return Err(e);
    }
    Ok(Creada {
        dir: if p.prefijo.is_empty() {
            raiz.clone()
        } else {
            raiz.join(&p.prefijo)
        },
        raiz,
        archivos: p.archivos.iter().map(|a| a.ruta.clone()).collect(),
        cuentas,
    })
}

fn guion_cuentas() -> &'static str {
    "guion-cuentas.toml"
}

/// The `nombre` of every `[[cuenta]]` of an accounts file.
fn nombres_declarados(text: &str) -> Result<Vec<String>, String> {
    let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    Ok(t.get("cuenta")
        .and_then(|c| c.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|c| c.get("nombre").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_parse_and_fail_by_name() {
        assert_eq!(
            parse_paleta("fondo=#0a0a0b, acento=e0322a").unwrap(),
            vec![
                ("fondo".to_string(), [10, 10, 11]),
                ("acento".to_string(), [224, 50, 42])
            ]
        );
        assert!(parse_paleta("fondo=#0a0a").unwrap_err().contains("fondo"));
    }
}
