//! R-0014 — accounts: the contract, the derived profile, and `guion cuenta
//! nueva` (one test or more per acceptance criterion). The template reel
//! itself is built and gated in guion-marca's `r0014_plantilla.rs`.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use guion_cuenta::cli::{banderas, preguntar, respuestas};
use guion_cuenta::contrato::Cuenta;
use guion_cuenta::nueva::{crear, plan, Respuestas};
use guion_cuenta::perfil;
use guion_reel::gate::Profile;
use guion_tablero::tablero::Tablero;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-cuenta-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn respuestas_arte() -> Respuestas {
    let mut r = Respuestas::con_defaults(
        "El extraño arte de computar",
        "Tecnología, IA y computación, desde cero",
        "computologo",
        "computación, IA y tecnología",
    );
    r.fecha = "2026-10-18".into();
    r.reglas =
        vec!["Un algoritmo se describe por lo que hace, no por la marca que lo vende.".into()];
    r
}

/// Every file under `d`, relative, with its bytes.
fn arbol(d: &Path) -> BTreeMap<String, Vec<u8>> {
    fn rec(base: &Path, d: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rec(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    rec(d, d, &mut out);
    out
}

// ---------- AC1: the contract ----------

#[test]
fn ac1_the_contract_parses_and_fails_by_name() {
    let p = plan(&respuestas_arte(), "").unwrap();
    let texto = &p
        .archivos
        .iter()
        .find(|a| a.ruta == "cuenta.toml")
        .unwrap()
        .texto;
    let c = Cuenta::parse(texto).expect("the generated contract holds");
    assert_eq!(c.cuenta.slug, "el-extrano-arte-de-computar");
    assert_eq!(c.cuenta.idioma, "es");
    assert_eq!(c.voz.modelo, "es-carlfm-x-low");
    assert_eq!(c.revisor.nombre, "computologo");
    assert_eq!(c.marca.roles.acento, ["acento"]);
    // an unknown key fails by name
    let typo = texto.replace("palabras = 8", "palbras = 8");
    assert!(Cuenta::parse(&typo).unwrap_err().contains("palbras"));
    // a role names a token that is not there
    let rol = texto.replace("texto = \"tinta\"", "texto = \"tinto\"");
    let e = Cuenta::parse(&rol).unwrap_err();
    assert!(
        e.contains("tinto") && e.contains("marca.roles.texto"),
        "{e}"
    );
    // the language is es or en
    let fr = texto.replace("idioma = \"es\"", "idioma = \"fr\"");
    assert!(Cuenta::parse(&fr).unwrap_err().contains("idioma"));
}

// ---------- AC2: the derived profile ----------

#[test]
fn ac2_the_gate_profile_is_derived_from_the_contract() {
    let mut r = respuestas_arte();
    r.paleta = vec![("acento".into(), [224, 50, 42])];
    let p = plan(&r, "").unwrap();
    let derivado = &p
        .archivos
        .iter()
        .find(|a| a.ruta == perfil::RUTA)
        .unwrap()
        .texto;
    assert_eq!(
        derivado,
        &perfil::perfil(&p.cuenta),
        "a pure function of the contract"
    );
    let g = Profile::parse(derivado).expect("guion reel check reads it");
    assert_eq!(g.brand.name, "El extraño arte de computar");
    assert_eq!(g.tokens["acento"], [224, 50, 42]);
    assert_eq!(g.roles.background, ["fondo", "panel"]);
    assert_eq!(g.roles.red, ["acento"]);
    assert_eq!(g.subtitle.font, "../brand/fonts/texto.ttf");
    assert_eq!(g.subtitle.center_x, 540.0);
    assert_eq!(g.caption.max_hashtags, 5);
    // the voice's forbidden words are refused like internal text
    assert!(g.caption.internal.iter().any(|w| w == "increíble"));
}

#[test]
fn ac2_perfil_check_catches_a_drift() {
    let d = dir("drift");
    let cuenta = d.join("cuenta");
    crear(&respuestas_arte(), &cuenta, None).unwrap();
    let args = |check: bool| {
        let mut a = vec!["perfil".to_string(), cuenta.display().to_string()];
        if check {
            a.push("--check".into());
        }
        a
    };
    let code = |a: &[String]| {
        format!(
            "{:?}",
            guion_cuenta::run(a, "2026-10-18", &|_| Ok(String::new()))
        )
    };
    let ok = format!("{:?}", std::process::ExitCode::SUCCESS);
    let fail = format!("{:?}", std::process::ExitCode::FAILURE);
    assert_eq!(code(&args(true)), ok, "fresh from the scaffold");
    let f = cuenta.join("cuenta.toml");
    let t = std::fs::read_to_string(&f).unwrap();
    std::fs::write(
        &f,
        t.replace("acento = [64, 140, 255]", "acento = [224, 50, 42]"),
    )
    .unwrap();
    assert_eq!(
        code(&args(true)),
        fail,
        "the contract changed, the profile did not"
    );
    assert_eq!(code(&args(false)), ok);
    assert_eq!(code(&args(true)), ok, "derived again");
}

// ---------- AC3: the scaffold ----------

#[test]
fn ac3_a_new_account_has_everything_the_contract_asks() {
    let d = dir("nueva");
    let cuenta = d.join("arte");
    let c = crear(&respuestas_arte(), &cuenta, None).expect("scaffolds");
    let slug = "el-extrano-arte-de-computar";
    for f in [
        "cuenta.toml",
        "brand/reel-perfil.toml",
        "CLAUDE.md",
        "docs/decisions.md",
        "estado.json",
        "guion-cuentas.toml",
        "motor/Cargo.toml",
        "motor/src/lib.rs",
        "plantillas/reel/escena.rs",
        "plantillas/reel/reel.toml",
        "plantillas/reel/voz.toml",
        "plantillas/reel/sonido.toml",
        "plantillas/reel/caption.md",
        "plantillas/reel/README.md",
        ".claude/agents/computologo.md",
        ".claude/agents/designer.md",
        &format!(".claude/skills/reel-{slug}/SKILL.md"),
        &format!("publicar/{slug}/perfil/README.md"),
    ] {
        assert!(
            c.archivos.iter().any(|a| a == f),
            "{f} missing from {:?}",
            c.archivos
        );
        assert!(cuenta.join(f).is_file(), "{f} not written");
    }
    let leer = |f: &str| std::fs::read_to_string(cuenta.join(f)).unwrap();
    // no hole left by the scaffold; the post's placeholders stay for `guion reel nuevo`
    for (f, _) in arbol(&cuenta) {
        assert!(!leer(&f).contains("{%"), "{f} keeps a {{% placeholder");
    }
    assert!(leer("plantillas/reel/escena.rs").contains("{{slug}}"));
    // the non-negotiable rules
    let claude = leer("CLAUDE.md");
    for regla in [
        "Nada se publica sin el dueño",
        "Todo número tiene procedencia",
        "Privacidad y consentimiento",
        "La materia se revisa antes que el diseño",
        "proceso-reel.md",
    ] {
        assert!(claude.contains(regla), "CLAUDE.md lacks «{regla}»");
    }
    // the reviewers, rendered from the generic templates
    let rev = leer(".claude/agents/computologo.md");
    assert!(rev.starts_with("---\nname: computologo\n"));
    assert!(rev.contains("computación, IA y tecnología"));
    assert!(rev.contains("no por la marca que lo vende"));
    assert!(leer(".claude/agents/designer.md").starts_with("---\nname: designer\n"));
    // the decisions log starts with its first row
    let dec = leer("docs/decisions.md");
    assert!(dec
        .lines()
        .any(|l| l.starts_with("| 2026-10-18 | Se crea la cuenta")));
    // the status file is JSON
    let e: serde_json::Value = serde_json::from_str(&leer("estado.json")).unwrap();
    assert_eq!(e["cuenta"], "El extraño arte de computar");
    assert!(e["piezas"].as_array().unwrap().is_empty());
    // the accounts file is the dashboard's, and a post comes out of the template
    guion_tablero::cuentas::parse(&leer("guion-cuentas.toml")).expect("the dashboard reads it");
    let t = Tablero::leer(std::slice::from_ref(&c.raiz), &|_| None).unwrap();
    let cu = t.cuenta("el extraño arte de computar").expect("listed");
    let n = guion_tablero::nuevo::crear(cu, "que-es-un-bit", Some("¿Qué es un bit?"), "2026-10-18")
        .expect("guion reel nuevo works on the scaffold");
    assert_eq!(n.dir, cuenta.join("posts/que-es-un-bit"));
    let motor = leer("motor/Cargo.toml");
    assert!(motor.contains("name = \"que-es-un-bit\"\npath = \"../posts/que-es-un-bit/escena.rs\""));
    let reel = std::fs::read_to_string(n.dir.join("reel.toml")).unwrap();
    assert!(
        reel.contains("bin = \"que-es-un-bit\"") && reel.contains("model = \"es-carlfm-x-low\"")
    );
    let t = Tablero::leer(&[c.raiz], &|_| None).unwrap();
    assert_eq!(t.cuentas[0].conteo.borrador, 1);
}

#[test]
fn ac3_same_answers_same_bytes() {
    let d = dir("det");
    crear(&respuestas_arte(), &d.join("a"), None).unwrap();
    crear(&respuestas_arte(), &d.join("b"), None).unwrap();
    let (a, b) = (arbol(&d.join("a")), arbol(&d.join("b")));
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    for (k, v) in &a {
        assert!(v == &b[k], "{k} differs between two runs");
    }
}

#[test]
fn ac3_flags_and_the_prompt_give_the_answers() {
    let args: Vec<String> = [
        "arte",
        "--nombre",
        "El extraño arte de computar",
        "--tema",
        "IA",
        "--revisor",
        "computologo",
        "--materia",
        "computación",
        "--regla",
        "uno",
        "--regla",
        "dos",
        "--idioma",
        "en",
        "--paleta",
        "acento=#e0322a",
        "--hashtags",
        "#ai #computing",
        "--repo",
        "..",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let b = banderas(&args).unwrap();
    assert_eq!(b.dir, Some(PathBuf::from("arte")));
    assert_eq!(b.repo, Some(PathBuf::from("..")));
    let r = respuestas(&b, "2026-10-18").unwrap();
    assert_eq!(r.reglas, ["uno", "dos"]);
    assert_eq!(r.idioma, "en");
    assert_eq!(r.paleta, [("acento".to_string(), [224, 50, 42])]);
    assert_eq!(r.hashtags, ["#ai", "#computing"]);
    assert_eq!(r.fecha, "2026-10-18");
    assert!(banderas(&["x".into(), "--nombr".into(), "y".into()])
        .unwrap_err()
        .contains("--nombr"));
    // the prompt asks only what is missing
    let mut r = Respuestas::con_defaults("", "", "", "");
    r.fecha = "2026-10-18".into();
    let mut entrada = Cursor::new("Mi cuenta\nUn tema\ncritico\nla materia\nuna regla\n\n");
    let mut salida = Vec::new();
    preguntar(&mut r, false, &mut entrada, &mut salida).unwrap();
    assert_eq!(
        (
            r.nombre.as_str(),
            r.slug.as_str(),
            r.tema.as_str(),
            r.revisor.as_str(),
            r.materia.as_str()
        ),
        ("Mi cuenta", "mi-cuenta", "Un tema", "critico", "la materia")
    );
    assert_eq!(r.reglas, ["una regla"]);
    assert!(plan(&r, "").is_ok());
}

// ---------- AC4: it never overwrites ----------

#[test]
fn ac4_refuses_to_overwrite_and_writes_nothing() {
    let d = dir("pisar");
    let cuenta = d.join("arte");
    std::fs::create_dir_all(&cuenta).unwrap();
    std::fs::write(cuenta.join("CLAUDE.md"), "mío").unwrap();
    let e = crear(&respuestas_arte(), &cuenta, None).unwrap_err();
    assert!(
        e.contains("refusing to overwrite") && e.contains("CLAUDE.md"),
        "{e}"
    );
    assert_eq!(arbol(&cuenta).len(), 1, "nothing else was written");
    assert_eq!(
        std::fs::read_to_string(cuenta.join("CLAUDE.md")).unwrap(),
        "mío"
    );
    // and a second scaffold on a whole account names every file it would touch
    let otra = d.join("otra");
    crear(&respuestas_arte(), &otra, None).unwrap();
    let antes = arbol(&otra);
    let e = crear(&respuestas_arte(), &otra, None).unwrap_err();
    assert!(
        e.contains("cuenta.toml") && e.contains("estado.json"),
        "{e}"
    );
    assert_eq!(arbol(&otra), antes);
}

// ---------- AC5: an account inside an existing repo ----------

#[test]
fn ac5_inside_a_repo_the_entry_is_appended_and_shared_paths_go_to_the_root() {
    let repo = dir("repo");
    let previo = "[[cuenta]]\nnombre = \"Rotor\"\nposts = [\"content/posts/*\"]\n";
    std::fs::write(repo.join("guion-cuentas.toml"), previo).unwrap();
    std::fs::create_dir_all(repo.join(".claude/agents")).unwrap();
    std::fs::write(repo.join(".claude/agents/designer.md"), "el de Rotor").unwrap();
    let c = crear(&respuestas_arte(), &repo.join("computar"), Some(&repo)).expect("scaffolds");
    let slug = "el-extrano-arte-de-computar";
    assert!(repo.join("computar/cuenta.toml").is_file());
    assert!(repo
        .join(format!(".claude/agents/designer-{slug}.md"))
        .is_file());
    assert_eq!(
        std::fs::read_to_string(repo.join(".claude/agents/designer.md")).unwrap(),
        "el de Rotor"
    );
    assert!(repo
        .join(format!("publicar/{slug}/perfil/README.md"))
        .is_file());
    assert!(!repo.join("computar/guion-cuentas.toml").exists());
    let cuentas = std::fs::read_to_string(repo.join("guion-cuentas.toml")).unwrap();
    assert!(cuentas.starts_with(previo), "appended, never rewritten");
    assert!(cuentas.contains("posts = [\"computar/posts/*\"]"));
    assert!(cuentas.contains("archivo = \"computar/motor/Cargo.toml\""));
    let motor = std::fs::read_to_string(repo.join("computar/motor/Cargo.toml")).unwrap();
    assert!(motor.contains("path = \"../../../guion-video-creator/crates/guion-marca\""));
    let t = Tablero::leer(std::slice::from_ref(&c.raiz), &|_| None).unwrap();
    assert_eq!(t.cuentas.len(), 2);
    // the same name twice is refused, and the file is left as it was
    let e = crear(&respuestas_arte(), &repo.join("otra"), Some(&repo)).unwrap_err();
    assert!(e.contains("already declared"), "{e}");
    assert_eq!(
        std::fs::read_to_string(repo.join("guion-cuentas.toml")).unwrap(),
        cuentas
    );
    assert!(!repo.join("otra").exists());
}
