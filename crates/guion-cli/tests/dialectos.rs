//! R-0007 — el CLI entiende los dos dialectos.
//!
//! Se prueba contra el binario de verdad y no contra las funciones por
//! dentro: el defecto que esto arregla era exactamente que el binario no
//! sabía algo que sus crates sí sabían, y una prueba que llama a la
//! función se lo habría perdido.

use std::path::{Path, PathBuf};
use std::process::Command;

fn raiz() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("la raíz del repo")
        .to_path_buf()
}

fn guion(args: &[&str]) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_guion");
    Command::new(exe)
        .args(args)
        .current_dir(raiz())
        .output()
        .expect("correr guion")
}

fn tmp(nombre: &str) -> PathBuf {
    let d = std::env::temp_dir().join("guion-r0007").join(nombre);
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// AC1. El guion declarativo que el CLI rechazaba con «unknown field
/// `comparacion`» ahora pasa `check`, y lo que imprime son sus renglones
/// con veredicto — que es lo que el creador necesita ver sin renderizar.
#[test]
fn check_acepta_el_dialecto_de_comparacion() {
    let out = guion(&["check", "guiones/reel40-gluteo.toml"]);
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "salió {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(txt.contains("Glúteo"), "{txt}");
    assert!(txt.contains("rumano"), "{txt}");
    assert!(txt.contains("hip thrust"), "{txt}");
    // Un veredicto por renglón, y el de este guion es «empatan»: 494 y
    // 451 quedan dentro del 15 % de empate. Que salga impreso es la
    // diferencia entre una comparación y una tabla.
    assert!(txt.contains("empatan"), "{txt}");
}

/// AC2. Y el dialecto de escena sigue funcionando igual: el cable nuevo
/// no puede haber costado el viejo.
#[test]
fn check_sigue_aceptando_el_dialecto_de_escena() {
    let out = guion(&["check", "templates/reel-09-biceps.screenplay.toml"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(txt.contains("reel-09-biceps"), "{txt}");
}

/// AC3. `render` sobre un guion de comparación escribe una secuencia
/// numerada de verdad — un archivo por cuadro en un solo directorio, no
/// un directorio por cuadro.
#[test]
fn render_escribe_una_secuencia_desde_una_comparacion() {
    let dir = tmp("render");
    let out = guion(&[
        "render",
        "guiones/reel40-gluteo.toml",
        "--out",
        dir.to_str().expect("utf8"),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut nombres: Vec<String> = std::fs::read_dir(&dir)
        .expect("el directorio existe")
        .map(|e| {
            e.expect("entrada")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    nombres.sort();
    assert!(nombres.len() >= 30, "sólo {} cuadros", nombres.len());
    assert_eq!(nombres[0], "frame_00000.ppm", "{nombres:?}");
    assert!(
        nombres.iter().all(|n| n.ends_with(".ppm")),
        "algo que no es un cuadro: {nombres:?}"
    );
}

/// AC4. Y lo dice: un comando que escribe cuadros y calla que falta el
/// molde entero deja creer que ya está.
#[test]
fn render_dice_lo_que_todavia_no_hace() {
    let dir = tmp("aviso");
    let out = guion(&[
        "render",
        "guiones/reel40-gluteo.toml",
        "--out",
        dir.to_str().expect("utf8"),
    ]);
    let txt = String::from_utf8_lossy(&out.stdout);
    assert!(txt.contains("sólo la gráfica"), "{txt}");
}

/// AC5. Un archivo sin ninguna de las dos tablas no se adivina: se
/// rechaza nombrando las dos que sí existen.
#[test]
fn un_archivo_sin_dialecto_se_rechaza_nombrando_los_dos() {
    let d = tmp("sin");
    std::fs::create_dir_all(&d).expect("mkdir");
    let f = d.join("raro.toml");
    std::fs::write(&f, "[otra_cosa]\nx = 1\n").expect("escribir");
    let out = guion(&["check", f.to_str().expect("utf8")]);
    assert!(!out.status.success(), "debería fallar");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("[meta]"), "{err}");
    assert!(err.contains("[comparacion]"), "{err}");
}

/// AC6. El error que sale es el del dialecto que el autor estaba
/// escribiendo. Un `[comparacion]` incompleto no puede reportar
/// «unknown field `comparacion`», que es el error del OTRO lector y fue
/// justo el que confundió durante meses.
#[test]
fn el_error_es_el_del_dialecto_que_se_estaba_escribiendo() {
    let d = tmp("roto");
    std::fs::create_dir_all(&d).expect("mkdir");
    let f = d.join("incompleto.toml");
    std::fs::write(&f, "[comparacion]\ntitulo = \"x\"\n").expect("escribir");
    let out = guion(&["check", f.to_str().expect("utf8")]);
    assert!(!out.status.success(), "debería fallar");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        !err.contains("unknown field `comparacion`"),
        "reportó el error del dialecto equivocado: {err}"
    );
    assert!(err.contains("no carga"), "{err}");
}
