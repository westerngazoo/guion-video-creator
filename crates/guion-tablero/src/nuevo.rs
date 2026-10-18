//! `guion reel nuevo <cuenta> <slug>`: a new post from the account's template
//! (R-0013 AC10).
//!
//! The template is a folder in the brand's repo (`plantilla` in
//! `guion-cuentas.toml`); this module only copies it. In file names and in
//! every UTF-8 file it replaces `{{slug}}`, `{{titulo}}`, `{{fecha}}` and
//! `{{cuenta}}`; it sets the README's estado to `borrador`; and it appends the
//! account's `registro` snippets (a scene binary in a manifest, say), so the
//! new post renders and the dashboard lists it at once.

use std::path::{Path, PathBuf};

use crate::post;
use crate::tablero::Cuenta;

/// The placeholders' values.
#[derive(Debug, Clone, PartialEq)]
pub struct Valores {
    pub slug: String,
    pub titulo: String,
    pub fecha: String,
    pub cuenta: String,
}

impl Valores {
    /// Replaces every placeholder; unknown `{{…}}` stay as they are.
    pub fn sustituir(&self, s: &str) -> String {
        s.replace("{{slug}}", &self.slug)
            .replace("{{titulo}}", &self.titulo)
            .replace("{{fecha}}", &self.fecha)
            .replace("{{cuenta}}", &self.cuenta)
    }
}

/// What was made.
#[derive(Debug, Clone, PartialEq)]
pub struct Creado {
    pub dir: PathBuf,
    /// Files written, relative to the new folder, sorted.
    pub archivos: Vec<String>,
    /// Repo files a snippet was appended to.
    pub registros: Vec<PathBuf>,
}

/// A slug is lowercase ASCII letters, digits and inner hyphens.
pub fn slug_valido(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && !s.ends_with('-')
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Copies the account's template into `<nuevos>/<slug>`.
pub fn crear(c: &Cuenta, slug: &str, titulo: Option<&str>, fecha: &str) -> Result<Creado, String> {
    if !slug_valido(slug) {
        return Err(format!(
            "«{slug}»: el slug va en minúsculas, cifras y guiones (fuerza-g)"
        ));
    }
    let plantilla = c
        .decl
        .plantilla
        .as_ref()
        .ok_or_else(|| format!("la cuenta «{}» no declara plantilla", c.nombre))?;
    let origen = c.raiz.join(plantilla);
    if !origen.is_dir() {
        return Err(format!("{}: la plantilla no existe", origen.display()));
    }
    let base = c.decl.carpeta_nuevos().ok_or_else(|| {
        format!(
            "la cuenta «{}»: ni `nuevos` ni un glob «…/*» en posts",
            c.nombre
        )
    })?;
    let destino = c.raiz.join(base).join(slug);
    if destino.exists() {
        return Err(format!("{}: ya existe", destino.display()));
    }
    let v = Valores {
        slug: slug.to_string(),
        titulo: titulo
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(slug)
            .to_string(),
        fecha: fecha.to_string(),
        cuenta: c.nombre.clone(),
    };
    let mut archivos = Vec::new();
    let r = copiar(&origen, &destino, Path::new(""), &v, &mut archivos)
        .and_then(|()| poner_borrador(&destino, &v));
    if let Err(e) = r {
        let _ = std::fs::remove_dir_all(&destino);
        return Err(e);
    }
    let mut registros = Vec::new();
    for reg in &c.decl.registro {
        let f = c.raiz.join(&reg.archivo);
        let texto = v.sustituir(&reg.texto);
        let actual = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        if actual.contains(texto.trim()) {
            continue;
        }
        let mut nuevo = actual;
        if !nuevo.ends_with('\n') {
            nuevo.push('\n');
        }
        if !texto.starts_with('\n') {
            nuevo.push('\n');
        }
        nuevo.push_str(&texto);
        if !nuevo.ends_with('\n') {
            nuevo.push('\n');
        }
        std::fs::write(&f, nuevo).map_err(|e| format!("{}: {e}", f.display()))?;
        registros.push(f);
    }
    archivos.sort();
    Ok(Creado {
        dir: destino,
        archivos,
        registros,
    })
}

fn copiar(
    origen: &Path,
    destino: &Path,
    rel: &Path,
    v: &Valores,
    archivos: &mut Vec<String>,
) -> Result<(), String> {
    std::fs::create_dir_all(destino).map_err(|e| format!("{}: {e}", destino.display()))?;
    let mut entradas: Vec<_> = std::fs::read_dir(origen)
        .map_err(|e| format!("{}: {e}", origen.display()))?
        .flatten()
        .collect();
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let nombre = v.sustituir(&e.file_name().to_string_lossy());
        let (de, a) = (e.path(), destino.join(&nombre));
        if de.is_dir() {
            copiar(&de, &a, &rel.join(&nombre), v, archivos)?;
            continue;
        }
        let bytes = std::fs::read(&de).map_err(|err| format!("{}: {err}", de.display()))?;
        let salida = match String::from_utf8(bytes) {
            Ok(t) => v.sustituir(&t).into_bytes(),
            Err(b) => b.into_bytes(),
        };
        std::fs::write(&a, salida).map_err(|err| format!("{}: {err}", a.display()))?;
        let permisos = std::fs::metadata(&de)
            .map_err(|err| format!("{}: {err}", de.display()))?
            .permissions();
        std::fs::set_permissions(&a, permisos).map_err(|err| format!("{}: {err}", a.display()))?;
        archivos.push(rel.join(&nombre).to_string_lossy().into_owned());
    }
    Ok(())
}

/// The README's estado becomes `borrador`; a README without the line gets one
/// under its title.
fn poner_borrador(dir: &Path, v: &Valores) -> Result<(), String> {
    let f = dir.join("README.md");
    let t = std::fs::read_to_string(&f)
        .map_err(|e| format!("{}: la plantilla no trae README.md ({e})", f.display()))?;
    let valor = format!("borrador ({}): recién creado de la plantilla", v.fecha);
    let mut lineas: Vec<String> = t.lines().map(str::to_string).collect();
    match post::leer_estado(&t) {
        Some(l) => lineas[l.linea] = format!("{}{valor}", l.prefijo),
        None => {
            let en = lineas
                .iter()
                .position(|l| l.starts_with("# "))
                .map_or(0, |i| i + 1);
            lineas.insert(en, String::new());
            lineas.insert(en + 1, format!("- **estado:** {valor}"));
        }
    }
    std::fs::write(&f, lineas.join("\n") + "\n").map_err(|e| format!("{}: {e}", f.display()))
}
