//! R-0013: the accounts dashboard, over a fixture workspace built in a temp
//! folder (no brand value: «Cuenta A», «Cuenta B»). One test per acceptance
//! criterion; the JSON state and one TUI screen are snapshots
//! (`GUION_SNAPSHOTS=1 cargo test -p guion-tablero` rewrites them).

use std::path::{Path, PathBuf};

use guion_tablero::acciones::{self, Abrir, Accion};
use guion_tablero::cuentas;
use guion_tablero::encargos;
use guion_tablero::nuevo;
use guion_tablero::post::{self, Estado};
use guion_tablero::tablero::Tablero;
use guion_tablero::ui::{self, App, Efecto};
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;

fn escribir(raiz: &Path, rel: &str, texto: &str) {
    let p = raiz.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, texto).unwrap();
}

const CUENTAS: &str = r#"
[[cuenta]]
nombre = "Cuenta A"
posts = ["posts/*"]
perfil = "marca/perfil.toml"
publicar = ["publicar/nivel-*"]
plantilla = "plantillas/reel"

[[cuenta.registro]]
archivo = "motor/Cargo.toml"
texto = """
# {{titulo}}
[[bin]]
name = "{{slug}}"
path = "../posts/{{slug}}/escena.rs"
"""

[[cuenta]]
nombre = "Cuenta B"
posts = ["b/posts/*"]
"#;

const CHECK_PASA: &str = r#"{"post": "posts/uno", "pass": true, "seconds": 1.0, "findings": [
  {"check": "sub-width", "pass": true, "detail": "2 lines, widest 500 px ≤ 840"},
  {"check": "loudness", "pass": true, "detail": "-14.1 LUFS"}]}"#;

const CHECK_FALLA: &str = r#"{"post": "posts/dos", "pass": false, "seconds": 1.0, "findings": [
  {"check": "sub-width", "pass": true, "detail": "3 lines, widest 700 px ≤ 840"},
  {"check": "red", "pass": false, "detail": "2 red regions", "at": ["1.00 s (f30)", "1.03 s (f31)"]}]}"#;

const REEL: &str = "[reel]\nbrand = \"../../marca/perfil.toml\"\nmax_duration = 90.0\n";

/// The fixture: two accounts, a narrated post that passes, one that fails,
/// a legacy one, one without estado, a folder without README, an encargo and
/// a template.
fn fixture(nombre: &str) -> PathBuf {
    let raiz = std::env::temp_dir().join(format!("guion-tablero-{}-{nombre}", std::process::id()));
    let _ = std::fs::remove_dir_all(&raiz);
    let r = &raiz;
    escribir(r, "guion-cuentas.toml", CUENTAS);
    escribir(r, "marca/perfil.toml", "# el perfil de la compuerta\n");
    // uno: narrado, con reel.toml (otro nombre), check que pasa, hoja, MP4 y entregables
    escribir(
        r,
        "posts/uno/README.md",
        "# ¿Por qué uno?\n\n- **estado:** listo (2026-10-01): todo bien\n- **nivel:** 1\n",
    );
    escribir(
        r,
        "posts/uno/reel.toml",
        &format!("{REEL}name = \"uno-reel\"\n[publish]\ndir = \"../../publicar/nivel-1/uno\"\n"),
    );
    escribir(r, "posts/uno/voz.toml", "[voz]\n");
    escribir(r, "posts/uno/check.json", CHECK_PASA);
    escribir(r, "posts/uno/uno-reel-audio.mp4", "");
    escribir(r, "posts/uno/cuadros/hoja.png", "");
    escribir(r, "publicar/nivel-1/uno/caption.txt", "");
    // dos: narrado, check que falla
    escribir(
        r,
        "posts/dos/README.md",
        "# ¿Y dos?\n\n- **estado:** revisión — falta fisico\n",
    );
    escribir(r, "posts/dos/reel.toml", REEL);
    escribir(r, "posts/dos/voz.toml", "[voz]\n");
    escribir(r, "posts/dos/check.json", CHECK_FALLA);
    escribir(r, "posts/dos/dos-audio.mp4", "");
    // tres: legado (sin reel.toml), publicado, MP4 mudo, entregables por el glob
    escribir(r, "posts/tres/README.md", "# Tres\n\nestado: Publicado.\n");
    escribir(r, "posts/tres/tres.mp4", "");
    escribir(r, "publicar/nivel-0/tres/tres.mp4", "");
    // sin README: no es un post
    escribir(r, "posts/notas/apuntes.md", "");
    // cuenta B: un post sin línea de estado
    escribir(
        r,
        "b/posts/cuatro/README.md",
        "# Cuatro\n\nSin estado todavía.\n",
    );
    escribir(
        r,
        "encargos.toml",
        "[[encargo]]\nid = \"E-0001\"\nfecha = \"2026-10-17\"\ncuenta = \"Cuenta A\"\nidea = \"¿Por qué cinco?\"\nestado = \"pendiente\"\n",
    );
    // la plantilla de la cuenta A
    escribir(
        r,
        "plantillas/reel/README.md",
        "# {{titulo}}\n\n- **estado:** idea\n- **creado:** {{fecha}} en {{cuenta}}\n",
    );
    escribir(
        r,
        "plantillas/reel/reel.toml",
        "[scene]\nbin = \"{{slug}}\"\n",
    );
    escribir(r, "plantillas/reel/{{slug}}.txt", "{{slug}}\n");
    escribir(
        r,
        "plantillas/reel/sub/escena.rs",
        "//! {{titulo}}\nfn main() {}\n",
    );
    escribir(r, "plantillas/reel/render.sh", "#!/bin/sh\necho {{slug}}\n");
    escribir(r, "motor/Cargo.toml", "[package]\nname = \"motor\"\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let p = r.join("plantillas/reel/render.sh");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    raiz
}

/// A fake ffmpeg: every MP4 lasts 42.5 s.
fn medir(_: &Path) -> Option<f64> {
    Some(42.5)
}

fn leer(raiz: &Path) -> Tablero {
    Tablero::leer(&[raiz.to_path_buf()], &medir).unwrap()
}

fn snapshot(nombre: &str, actual: &str) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(nombre);
    if std::env::var_os("GUION_SNAPSHOTS").is_some() || !p.exists() {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, actual).unwrap();
    }
    let esperado = std::fs::read_to_string(&p).unwrap();
    assert!(esperado == actual, "{nombre} changed:\n{actual}");
}

#[test]
fn ac1_the_accounts_file_is_the_repos_and_fails_by_name() {
    let c = cuentas::parse(CUENTAS).unwrap();
    assert_eq!(c.cuentas.len(), 2);
    assert_eq!(c.cuentas[0].nombre, "Cuenta A");
    assert_eq!(c.cuentas[0].carpeta_nuevos().as_deref(), Some("posts"));
    assert_eq!(c.cuentas[1].perfil, None);
    let e =
        cuentas::parse("[[cuenta]]\nnombre = \"A\"\nposts = [\"p/*\"]\npostz = 1\n").unwrap_err();
    assert!(e.contains("postz"), "{e}");
    let e = cuentas::parse("[[cuenta]]\nnombre = \"A\"\nposts = [\"p/*\"]\n[[cuenta]]\nnombre = \"a\"\nposts = [\"q/*\"]\n").unwrap_err();
    assert!(e.contains("twice"), "{e}");
    let e = cuentas::parse("[[cuenta]]\nnombre = \"A\"\nposts = [\"/abs/*\"]\n").unwrap_err();
    assert!(e.contains("/abs/*"), "{e}");
    assert!(cuentas::parse("cuenta = []\n").is_err());
    let e = Tablero::leer(&[PathBuf::from("/no/such/repo")], &medir).unwrap_err();
    assert!(e.contains("guion-cuentas.toml"), "{e}");
}

#[test]
fn ac2_a_post_is_read_from_its_folder() {
    let raiz = fixture("ac2");
    let t = leer(&raiz);
    let a = &t.cuentas[0];
    let nombres: Vec<&str> = a.posts.iter().map(|p| p.nombre.as_str()).collect();
    assert_eq!(
        nombres,
        ["dos", "tres", "uno"],
        "sorted, and a folder without README is no post"
    );
    let uno = &a.posts[2];
    assert_eq!(uno.titulo.as_deref(), Some("¿Por qué uno?"));
    assert_eq!(uno.estado, Some(Estado::Listo));
    assert!(uno.reel && uno.narrado);
    assert_eq!(
        uno.video.as_deref(),
        Some("posts/uno/uno-reel-audio.mp4"),
        "reel.toml's name"
    );
    assert_eq!(uno.duracion, Some(42.5));
    assert_eq!(uno.hoja.as_deref(), Some("posts/uno/cuadros/hoja.png"));
    assert_eq!(
        uno.publicar.as_deref(),
        Some("publicar/nivel-1/uno"),
        "reel.toml's [publish]"
    );
    let c = uno.check.as_ref().unwrap();
    assert!(c.pass && c.total == 2 && c.fallas == 0);
    let dos = &a.posts[0];
    assert_eq!(dos.estado, Some(Estado::Revision));
    let c = dos.check.as_ref().unwrap();
    assert!(!c.pass && c.fallas == 1);
    assert!(
        c.lineas[1].starts_with("FAIL red") && c.lineas[1].contains("@ 1.00 s (f30); 1.03 s (f31)"),
        "{:?}",
        c.lineas
    );
    let tres = &a.posts[1];
    assert_eq!(
        (tres.estado, tres.reel, tres.tipo()),
        (Some(Estado::Publicado), false, "legado")
    );
    assert_eq!(
        tres.video.as_deref(),
        Some("posts/tres/tres.mp4"),
        "a silent MP4 when there is no -audio"
    );
    assert_eq!(
        tres.publicar.as_deref(),
        Some("publicar/nivel-0/tres"),
        "the account's publicar glob"
    );
    let cuatro = &t.cuentas[1].posts[0];
    assert_eq!(
        (cuatro.estado, cuatro.nota.clone(), cuatro.check.clone()),
        (None, None, None)
    );
    // the estado line, alone
    let l = post::leer_estado("x\n* **Estado**: borrador, falta todo\n").unwrap();
    assert_eq!((l.linea, l.estado), (1, Some(Estado::Borrador)));
    assert_eq!(post::leer_check("{\"pass\": true}").unwrap().total, 0);
    assert!(post::leer_check("{").is_err());
}

#[test]
fn ac3_the_whole_state_as_json() {
    let raiz = fixture("ac3");
    let t = leer(&raiz);
    let a = &t.cuentas[0];
    assert_eq!(
        (
            a.conteo.listo,
            a.conteo.revision,
            a.conteo.publicado,
            a.conteo.legado
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(t.cuentas[1].conteo.sin_estado, 1);
    assert_eq!(t.filtrar(Some("cuenta a"), Some(Estado::Listo)).len(), 1);
    let json = t.to_json().replace(&raiz.display().to_string(), "<RAIZ>");
    assert_eq!(
        json,
        leer(&raiz)
            .to_json()
            .replace(&raiz.display().to_string(), "<RAIZ>"),
        "deterministic"
    );
    snapshot("estado.json", &json);
}

fn pantalla(app: &App, w: u16, h: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| ui::dibujar(f, app)).unwrap();
    let buf = term.backend().buffer().clone();
    let mut s = String::new();
    for y in 0..h {
        let mut linea = String::new();
        for x in 0..w {
            linea.push_str(buf[(x, y)].symbol());
        }
        s.push_str(linea.trim_end());
        s.push('\n');
    }
    s
}

fn tecla(app: &mut App, c: KeyCode) -> Vec<Efecto> {
    app.tecla(KeyEvent::new(c, KeyModifiers::NONE))
}

#[test]
fn ac4_the_screen_three_panes_a_log_and_the_keys() {
    let raiz = fixture("ac4");
    let mut app = App::new(leer(&raiz), PathBuf::from("guion"), "2026-10-18".into());
    app.pantalla = false;
    // the cursor on «uno» (third), marked; the filter cycles and comes back
    tecla(&mut app, KeyCode::Down);
    tecla(&mut app, KeyCode::Down);
    tecla(&mut app, KeyCode::Char(' '));
    assert_eq!(app.marcados.len(), 1);
    for _ in 0..6 {
        tecla(&mut app, KeyCode::Char('f'));
    }
    assert_eq!(app.filtro, None);
    tecla(&mut app, KeyCode::Char('f'));
    tecla(&mut app, KeyCode::Char('f'));
    tecla(&mut app, KeyCode::Char('f'));
    assert_eq!(app.filtro, Some(Estado::Revision));
    assert_eq!(app.visibles().len(), 1);
    for _ in 0..3 {
        tecla(&mut app, KeyCode::Char('f'));
    }
    app.post_sel = 2;
    app.registrar("[uno] $ guion reel check posts/uno --brand marca/perfil.toml");
    let s = pantalla(&app, 140, 34);
    assert!(
        s.contains("Cuenta A") && s.contains("¿Por qué uno?") && s.contains("FAIL 1"),
        "{s}"
    );
    snapshot("tablero.txt", &s);
}

#[test]
fn ac5_actions_are_the_existing_commands() {
    let raiz = fixture("ac5");
    let t = leer(&raiz);
    let (a, b) = (&t.cuentas[0], &t.cuentas[1]);
    let (dos, tres, uno) = (&a.posts[0], &a.posts[1], &a.posts[2]);
    let g = Path::new("/bin/guion");
    let d = |p: &guion_tablero::post::Post| p.dir.display().to_string();
    let c = acciones::comando(Accion::Render, uno, a, g).unwrap();
    assert_eq!(
        (c.programa.as_path(), c.args.clone(), c.dir.clone()),
        (g, vec!["reel".to_string(), d(uno)], raiz.clone())
    );
    let c = acciones::comando(Accion::Voz, dos, a, g).unwrap();
    assert_eq!(c.args, ["reel", "voice", &d(dos)]);
    let c = acciones::comando(Accion::Check, uno, a, g).unwrap();
    let perfil = raiz.join("marca/perfil.toml").display().to_string();
    assert_eq!(c.args, ["reel", "check", &d(uno), "--brand", &perfil]);
    assert!(c.mostrar().starts_with("/bin/guion reel check "));
    // a legacy post: no render, no voice; the check still runs with the profile
    let e = acciones::comando(Accion::Render, tres, a, g).unwrap_err();
    assert!(e.contains("legado"), "{e}");
    assert!(acciones::comando(Accion::Voz, tres, a, g).is_err());
    assert!(acciones::comando(Accion::Check, tres, a, g).is_ok());
    // no profile and no reel.toml: the check cannot run
    assert!(acciones::comando(Accion::Check, &b.posts[0], b, g).is_err());
    assert_eq!(Accion::parse("voice"), Some(Accion::Voz));
    // batch: marked posts, one job each; the legacy one is skipped in the log
    let mut app = App::new(t.clone(), g.to_path_buf(), "2026-10-18".into());
    tecla(&mut app, KeyCode::Char('a'));
    let ef = tecla(&mut app, KeyCode::Char('r'));
    let [Efecto::Lanzar(trabajos)] = ef.as_slice() else {
        panic!("{ef:?}")
    };
    assert_eq!(trabajos.len(), 2);
    assert!(app.log.iter().any(|l| l.contains("legado")));
    // the jobs really run, in the background, and report their end
    let (tx, rx) = std::sync::mpsc::channel();
    let echo = acciones::Comando {
        programa: "sh".into(),
        args: vec!["-c".into(), "echo hola; exit 3".into()],
        dir: raiz.clone(),
    };
    let t1 = guion_tablero::trabajos::Trabajo {
        accion: Accion::Check,
        post: "uno".into(),
        dir: uno.dir.clone(),
        comando: echo,
    };
    guion_tablero::trabajos::lanzar(vec![t1], 2, tx);
    let eventos: Vec<_> = rx.iter().collect();
    assert!(eventos.iter().any(
        |e| matches!(e, guion_tablero::trabajos::Evento::Linea { texto, .. } if texto == "hola")
    ));
    assert!(matches!(
        eventos.last(),
        Some(guion_tablero::trabajos::Evento::Fin {
            ok: false,
            codigo: Some(3),
            ..
        })
    ));
}

#[test]
fn ac6_open_the_sheet_or_say_where_it_is() {
    let raiz = fixture("ac6");
    let t = leer(&raiz);
    let (a, uno) = (&t.cuentas[0], &t.cuentas[0].posts[2]);
    assert_eq!(acciones::abridor("macos", false), Some("open"));
    assert_eq!(acciones::abridor("linux", true), Some("xdg-open"));
    assert_eq!(acciones::abridor("linux", false), None);
    let hoja = raiz.join("posts/uno/cuadros/hoja.png");
    assert_eq!(
        acciones::abrir(uno, a, "linux", false).unwrap(),
        Abrir::Ruta(hoja.clone())
    );
    let Abrir::Con(c) = acciones::abrir(uno, a, "linux", true).unwrap() else {
        panic!()
    };
    assert_eq!(
        (c.programa, c.args),
        (PathBuf::from("xdg-open"), vec![hoja.display().to_string()])
    );
    // without a sheet, the MP4
    let tres = &a.posts[1];
    assert_eq!(
        acciones::abrir(tres, a, "linux", false).unwrap(),
        Abrir::Ruta(raiz.join("posts/tres/tres.mp4"))
    );
    assert!(acciones::abrir(&t.cuentas[1].posts[0], &t.cuentas[1], "linux", false).is_err());
}

#[test]
fn ac7_mark_publicado_rewrites_the_estado_line() {
    let r = acciones::marcar_publicado(
        "# T\n\n- **estado:** listo (2026-10-01): todo bien.\n- **nivel:** 1\n",
        "2026-10-18",
        "https://www.instagram.com/reel/x/",
    )
    .unwrap();
    assert_eq!(r, "# T\n\n- **estado:** publicado (2026-10-18, https://www.instagram.com/reel/x/). Antes: listo (2026-10-01): todo bien\n- **nivel:** 1\n");
    assert!(acciones::marcar_publicado("estado: listo", "18/10/2026", "").is_err());
    assert!(acciones::marcar_publicado("# Sin estado\n", "2026-10-18", "").is_err());
    let raiz = fixture("ac7");
    let t = leer(&raiz);
    acciones::publicar(&t.cuentas[0].posts[0], "2026-10-18", "").unwrap();
    let dos = &leer(&raiz).cuentas[0].posts[0];
    assert_eq!(dos.estado, Some(Estado::Publicado));
    assert_eq!(
        dos.nota.as_deref(),
        Some("publicado (2026-10-18). Antes: revisión — falta fisico")
    );
}

#[test]
fn ac8_an_encargo_is_appended() {
    let raiz = fixture("ac8");
    let t = leer(&raiz);
    let e = t
        .encargar("cuenta b", "¿Por qué \"seis\"?", "2026-10-18", None)
        .unwrap();
    assert_eq!(
        (e.id.as_str(), e.cuenta.as_str(), e.estado.as_str()),
        ("E-0002", "Cuenta B", "pendiente")
    );
    let todos = encargos::load(&raiz).unwrap();
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[1].idea, "¿Por qué \"seis\"?");
    assert!(
        std::fs::read_to_string(raiz.join("encargos.toml"))
            .unwrap()
            .starts_with("[[encargo]]\nid = \"E-0001\""),
        "appended, not rewritten"
    );
    assert!(t
        .encargar("Cuenta Z", "x", "2026-10-18", None)
        .unwrap_err()
        .contains("Cuenta Z"));
    assert!(t
        .encargar("Cuenta A", "dos\nlíneas", "2026-10-18", None)
        .is_err());
    assert!(t.encargar("Cuenta A", "  ", "2026-10-18", None).is_err());
    // a repo without the file gets one
    let otra = fixture("ac8-nuevo");
    std::fs::remove_file(otra.join("encargos.toml")).unwrap();
    let e = leer(&otra)
        .encargar("Cuenta A", "idea", "2026-10-18", Some("mi-post"))
        .unwrap();
    assert_eq!(
        (e.id.as_str(), e.post.as_deref()),
        ("E-0001", Some("mi-post"))
    );
    assert_eq!(leer(&otra).encargos.len(), 1);
}

#[test]
fn ac9_lote_runs_an_action_over_a_filtered_set() {
    let raiz = fixture("ac9");
    let t = leer(&raiz);
    let g = Path::new("guion");
    let (tr, om) =
        guion_tablero::plan_lote(&t, Accion::Check, Some("cuenta a"), Some(Estado::Listo), g)
            .unwrap();
    assert_eq!((tr.len(), om.len()), (1, 0));
    assert_eq!(tr[0].post, "uno");
    let (tr, om) = guion_tablero::plan_lote(&t, Accion::Render, Some("Cuenta A"), None, g).unwrap();
    assert_eq!((tr.len(), om.len()), (2, 1), "the legacy post is skipped");
    let (tr, om) = guion_tablero::plan_lote(&t, Accion::Check, None, None, g).unwrap();
    assert_eq!((tr.len(), om.len()), (3, 1), "cuenta B has no profile");
    assert!(
        guion_tablero::plan_lote(&t, Accion::Check, Some("nadie"), None, g)
            .unwrap_err()
            .contains("Cuenta A")
    );
    let args: Vec<String> = [
        "--workspace",
        "a",
        "b",
        "lote",
        "render",
        "--cuenta",
        "X",
        "--estado",
        "revision",
        "-j",
        "3",
        "--dry-run",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let o = guion_tablero::opciones(&args).unwrap();
    assert_eq!(o.workspaces, [PathBuf::from("a"), PathBuf::from("b")]);
    assert_eq!(o.resto, ["lote", "render"]);
    assert_eq!(
        (o.cuenta.as_deref(), o.estado, o.paralelo, o.dry_run),
        (Some("X"), Some(Estado::Revision), 3, true)
    );
    assert!(guion_tablero::opciones(&["--estado".into(), "quizá".into()]).is_err());
    assert!(guion_tablero::opciones(&["--paralelo".into(), "0".into()]).is_err());
}

#[test]
fn ac10_a_new_post_from_the_accounts_template() {
    let raiz = fixture("ac10");
    let t = leer(&raiz);
    let a = &t.cuentas[0];
    let n = nuevo::crear(a, "mi-post", Some("¿Mi título?"), "2026-10-18").unwrap();
    assert_eq!(n.dir, raiz.join("posts/mi-post"));
    assert_eq!(
        n.archivos,
        [
            "README.md",
            "mi-post.txt",
            "reel.toml",
            "render.sh",
            "sub/escena.rs"
        ]
    );
    let leer_a = |r: &str| std::fs::read_to_string(n.dir.join(r)).unwrap();
    assert_eq!(leer_a("README.md"), "# ¿Mi título?\n\n- **estado:** borrador (2026-10-18): recién creado de la plantilla\n- **creado:** 2026-10-18 en Cuenta A\n");
    assert_eq!(leer_a("reel.toml"), "[scene]\nbin = \"mi-post\"\n");
    assert_eq!(leer_a("mi-post.txt"), "mi-post\n");
    assert_eq!(leer_a("sub/escena.rs"), "//! ¿Mi título?\nfn main() {}\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let modo = std::fs::metadata(n.dir.join("render.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(modo & 0o111, 0o111, "render.sh stays executable");
    }
    let motor = std::fs::read_to_string(raiz.join("motor/Cargo.toml")).unwrap();
    assert!(
        motor.ends_with(
            "\n# ¿Mi título?\n[[bin]]\nname = \"mi-post\"\npath = \"../posts/mi-post/escena.rs\"\n"
        ),
        "{motor}"
    );
    assert_eq!(n.registros, [raiz.join("motor/Cargo.toml")]);
    // the dashboard lists it at once, as borrador
    let t2 = leer(&raiz);
    let p = t2.cuentas[0]
        .posts
        .iter()
        .find(|p| p.nombre == "mi-post")
        .unwrap();
    assert_eq!(p.estado, Some(Estado::Borrador));
    // twice, a bad slug, an account without template: errors that say why
    assert!(nuevo::crear(a, "mi-post", None, "2026-10-18")
        .unwrap_err()
        .contains("ya existe"));
    assert!(nuevo::crear(a, "Mi Post", None, "2026-10-18")
        .unwrap_err()
        .contains("slug"));
    assert!(nuevo::crear(&t.cuentas[1], "x", None, "2026-10-18")
        .unwrap_err()
        .contains("plantilla"));
    // no title: the slug
    let n2 = nuevo::crear(a, "otro", None, "2026-10-18").unwrap();
    assert!(std::fs::read_to_string(n2.dir.join("README.md"))
        .unwrap()
        .starts_with("# otro\n"));
    // the TUI's `e` with a slug: post and encargo in one go
    let mut app = App::new(leer(&raiz), PathBuf::from("guion"), "2026-10-18".into());
    tecla(&mut app, KeyCode::Char('e'));
    for ch in "¿Tres?".chars() {
        tecla(&mut app, KeyCode::Char(ch));
    }
    tecla(&mut app, KeyCode::Enter);
    for ch in "tercero".chars() {
        tecla(&mut app, KeyCode::Char(ch));
    }
    assert_eq!(tecla(&mut app, KeyCode::Enter), [Efecto::Releer]);
    assert!(raiz.join("posts/tercero/README.md").is_file());
    let ultimo = encargos::load(&raiz).unwrap().pop().unwrap();
    assert_eq!(
        (
            ultimo.cuenta.as_str(),
            ultimo.idea.as_str(),
            ultimo.post.as_deref()
        ),
        ("Cuenta A", "¿Tres?", Some("tercero"))
    );
}

#[test]
fn ac11_an_encargo_carries_the_owners_details() {
    let raiz = fixture("ac11");
    let t = leer(&raiz);
    let d = encargos::Detalles {
        formato: Some(" Short ".into()),
        prioridad: Some("alta".into()),
        para: Some("2026-10-25".into()),
        nivel: Some("1.3".into()),
        variantes: vec!["pegado".into(), "inclinado \"atrás\"".into()],
        remake: Some("31-triceps".into()),
        notas: Some("Fuente: la del reel viejo.\nSin músculos.".into()),
        post: None,
    };
    let e = t
        .encargar_con("Cuenta A", "¿Pegado o inclinado?", "2026-10-22", &d)
        .unwrap();
    assert_eq!(e.formato.as_deref(), Some("short"), "trimmed, lower case");
    let todos = encargos::load(&raiz).unwrap();
    assert_eq!(todos.last(), Some(&e), "what was written reads back");
    assert_eq!(e.variantes, ["pegado", "inclinado \"atrás\""]);
    assert_eq!(
        e.notas.as_deref(),
        Some("Fuente: la del reel viejo.\nSin músculos.")
    );
    // an encargo with no details writes none of their keys
    let solo = t
        .encargar("Cuenta A", "¿Y siete?", "2026-10-22", None)
        .unwrap();
    let texto = std::fs::read_to_string(raiz.join("encargos.toml")).unwrap();
    let bloque = texto.rsplit("[[encargo]]").next().unwrap();
    assert!(
        bloque.contains(&solo.id) && !bloque.contains("formato") && !bloque.contains("variantes")
    );
    // the closed fields and the date are checked, and nothing is written
    for (malo, campo) in [
        (
            encargos::Detalles {
                formato: Some("tiktok".into()),
                ..Default::default()
            },
            "formato",
        ),
        (
            encargos::Detalles {
                prioridad: Some("urgente".into()),
                ..Default::default()
            },
            "prioridad",
        ),
        (
            encargos::Detalles {
                para: Some("25/10/2026".into()),
                ..Default::default()
            },
            "para",
        ),
        (
            encargos::Detalles {
                variantes: vec!["solo una".into()],
                ..Default::default()
            },
            "variantes",
        ),
        (
            encargos::Detalles {
                nivel: Some("dos\nlíneas".into()),
                ..Default::default()
            },
            "nivel",
        ),
        (
            encargos::Detalles {
                post: Some("no-existe".into()),
                ..Default::default()
            },
            "no-existe",
        ),
    ] {
        let err = t
            .encargar_con("Cuenta A", "x", "2026-10-22", &malo)
            .unwrap_err();
        assert!(err.contains(campo), "{campo}: {err}");
    }
    assert_eq!(
        encargos::load(&raiz).unwrap().len(),
        3,
        "the bad ones wrote nothing"
    );
    // empty text is no field
    let vacios = encargos::Detalles {
        formato: Some(" ".into()),
        variantes: vec![" ".into(), "".into()],
        ..Default::default()
    };
    let e = t
        .encargar_con("Cuenta A", "¿Ocho?", "2026-10-22", &vacios)
        .unwrap();
    assert!(e.formato.is_none() && e.variantes.is_empty());
}

#[test]
fn ac12_a_taken_encargo_follows_its_post() {
    let raiz = fixture("ac12");
    // a comment and an unknown-to-us order of keys must survive the rewrite
    escribir(
        &raiz,
        "encargos.toml",
        "# mis encargos\n\n[[encargo]]\nid = \"E-0001\"\nfecha = \"2026-10-17\"\ncuenta = \"Cuenta A\"\nidea = \"¿Por qué cinco?\"\nestado = \"pendiente\"\n# nota del agente\n\n[[encargo]]\nid = \"E-0002\"\nfecha = \"2026-10-18\"\ncuenta = \"Cuenta A\"\nidea = \"¿Y dos?\"\nestado = \"pendiente\"\nformato = \"reel\"\n\n[[encargo]]\nid = \"E-0003\"\nfecha = \"2026-10-18\"\ncuenta = \"Cuenta A\"\nidea = \"¿Tres?\"\nestado = \"pendiente\"\n",
    );
    let t = leer(&raiz);
    assert_eq!(
        t.encargos[0].vivo, "pendiente",
        "no post: the stored estado"
    );
    // taking it links the post (by its folder's name or its path)
    let e = t.marcar("E-0001", "en curso", Some("uno"), None).unwrap();
    assert_eq!(
        (e.estado.as_str(), e.post.as_deref()),
        ("en curso", Some("uno"))
    );
    encargos::tomar(&raiz, "E-0002", "posts/dos").unwrap();
    let texto = std::fs::read_to_string(raiz.join("encargos.toml")).unwrap();
    assert!(texto.starts_with("# mis encargos\n") && texto.contains("# nota del agente"));
    assert!(
        texto.contains("estado = \"en curso\"\npost = \"posts/dos\"\nformato = \"reel\""),
        "{texto}"
    );
    // the third block is untouched
    assert!(texto.ends_with("idea = \"¿Tres?\"\nestado = \"pendiente\"\n"));
    // the dashboard shows the post's estado: uno is listo, dos is in revisión
    let t = leer(&raiz);
    let vivo: Vec<(&str, &str, Option<&str>)> = t
        .encargos
        .iter()
        .map(|e| (e.encargo.id.as_str(), e.vivo.as_str(), e.paquete.as_deref()))
        .collect();
    assert_eq!(
        vivo,
        [
            ("E-0001", "listo", Some("publicar/nivel-1/uno")),
            ("E-0002", "en curso", None),
            ("E-0003", "pendiente", None),
        ]
    );
    let c = t.cuenta("Cuenta A").unwrap();
    assert_eq!(t.pedidos_de(c).count(), 3);
    // the post publishes: the encargo follows with no one touching it
    let uno = guion_tablero::tablero::post_de(t.cuenta("Cuenta A").unwrap(), "uno").unwrap();
    acciones::publicar(uno, "2026-10-20", "").unwrap();
    assert_eq!(leer(&raiz).encargos[0].vivo, "publicado");
    // descartado stays descartado
    let t = leer(&raiz);
    t.marcar("E-0002", "descartado", None, None).unwrap();
    assert_eq!(leer(&raiz).encargos[1].vivo, "descartado");
    // errors name what is wrong, and write nothing
    let antes = std::fs::read_to_string(raiz.join("encargos.toml")).unwrap();
    let t = leer(&raiz);
    assert!(t
        .marcar("E-0009", "hecho", None, None)
        .unwrap_err()
        .contains("E-0009"));
    assert!(t
        .marcar("E-0003", "listo", None, None)
        .unwrap_err()
        .contains("estado"));
    assert!(
        t.marcar("E-0003", "en curso", Some("cuatro"), None)
            .unwrap_err()
            .contains("cuatro"),
        "cuatro is Cuenta B's"
    );
    assert_eq!(
        std::fs::read_to_string(raiz.join("encargos.toml")).unwrap(),
        antes
    );
}

#[test]
fn ac12_reel_nuevo_takes_the_encargo_and_the_command_line_moves_it() {
    let raiz = fixture("ac12-cli");
    let r = raiz.display().to_string();
    let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // an encargo of another account is refused before anything is made
    guion_tablero::run_nuevo(&a(&[
        "Cuenta B",
        "seis",
        "--encargo",
        "E-0001",
        "--repo",
        &r,
    ]));
    assert!(!raiz.join("b/posts/seis").exists());
    // reel nuevo makes the post and takes the encargo for it
    guion_tablero::run_nuevo(&a(&[
        "Cuenta A",
        "cinco",
        "--titulo",
        "¿Por qué cinco?",
        "--encargo",
        "E-0001",
        "--repo",
        &r,
    ]));
    assert!(raiz.join("posts/cinco/README.md").is_file());
    let e = &encargos::load(&raiz).unwrap()[0];
    assert_eq!(
        (e.estado.as_str(), e.post.as_deref()),
        ("en curso", Some("cinco"))
    );
    assert_eq!(
        leer(&raiz).encargos[0].vivo,
        "en curso",
        "the template's README says idea"
    );
    // `guion encargo nuevo` with details, then `marcar` with a dash for the space
    guion_tablero::run_encargo(&a(&[
        "nuevo",
        "Cuenta B",
        "¿Por",
        "qué",
        "seis?",
        "--formato",
        "youtube",
        "--variante",
        "A",
        "--variante",
        "B",
        "--repo",
        &r,
    ]));
    let todos = encargos::load(&raiz).unwrap();
    let e = todos.last().unwrap();
    assert_eq!(
        (e.id.as_str(), e.idea.as_str(), e.formato.as_deref()),
        ("E-0002", "¿Por qué seis?", Some("youtube"))
    );
    assert_eq!(e.variantes, ["A", "B"]);
    guion_tablero::run_encargo(&a(&["marcar", "E-0002", "en-curso", "--repo", &r]));
    assert_eq!(encargos::load(&raiz).unwrap()[1].estado, "en curso");
}
