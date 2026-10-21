//! The generic templates a new account is rendered from (R-0014 AC4). They
//! live in this repo, in `templates/cuenta/`, and are compiled in, so the
//! binary scaffolds the same files wherever it runs.
//!
//! `{%clave%}` is filled at scaffold time ([`crate::texto::sustituir`]);
//! `{{slug}}`, `{{titulo}}`, `{{fecha}}` and `{{cuenta}}` stay for
//! `guion reel nuevo` (R-0013 AC10) to fill when a post is made from the
//! account's template.

use crate::texto::sustituir;

macro_rules! t {
    ($p:literal) => {
        include_str!(concat!("../../../templates/cuenta/", $p))
    };
}

/// (where it goes, relative to the account; or a role: `agente-revisor`,
/// `agente-designer`, `skill`, `publicar-perfil`) → the template.
pub const TODAS: &[(&str, &str)] = &[
    ("CLAUDE.md", t!("CLAUDE.md")),
    (".gitignore", t!("gitignore")),
    ("docs/decisions.md", t!("docs/decisions.md")),
    ("brand/fonts/README.md", t!("docs/fuentes.md")),
    // A real `Cargo.toml` in this repo would be read by cargo as a package (and
    // `{%slug%}` is no valid package name): any repo with this one as a git
    // dependency would fail to resolve. So the template is `.plantilla`.
    ("motor/Cargo.toml", t!("motor/Cargo.toml.plantilla")),
    ("motor/src/lib.rs", t!("motor/src/lib.rs")),
    ("plantillas/reel/README.md", t!("plantilla/README.md")),
    ("plantillas/reel/caption.md", t!("plantilla/caption.md")),
    ("plantillas/reel/escena.rs", t!("plantilla/escena.rs")),
    ("plantillas/reel/reel.toml", t!("plantilla/reel.toml")),
    ("plantillas/reel/sonido.toml", t!("plantilla/sonido.toml")),
    ("plantillas/reel/voz.toml", t!("plantilla/voz.toml")),
    ("posts/README.md", t!("docs/posts.md")),
    ("agente-revisor", t!("agentes/revisor.md")),
    ("agente-designer", t!("agentes/designer.md")),
    ("skill", t!("skill/SKILL.md")),
    ("publicar-perfil", t!("docs/publicar-perfil.md")),
];

/// One template, filled.
pub fn render(plantilla: &str, valores: &[(&str, String)], nombre: &str) -> Result<String, String> {
    sustituir(plantilla, valores, nombre)
}

#[cfg(test)]
mod sin_manifiestos {
    /// No template may be named `Cargo.toml`: cargo would read it as a package
    /// of this repo, and a git dependency on guion-video-creator would fail.
    #[test]
    fn ninguna_plantilla_se_llama_cargo_toml() {
        fn buscar(d: &std::path::Path, malos: &mut Vec<String>) {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    buscar(&p, malos);
                } else if p.file_name().is_some_and(|n| n == "Cargo.toml") {
                    malos.push(p.display().to_string());
                }
            }
        }
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../templates");
        let mut malos = Vec::new();
        buscar(&raiz, &mut malos);
        assert!(malos.is_empty(), "{malos:?}");
    }
}
