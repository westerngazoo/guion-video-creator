//! R-0008 — la marca del guion llega al cuadro, y la que no existe truena.
//!
//! Se prueba contra el binario de verdad, como en `dialectos.rs`: el
//! defecto que esto cierra era del binario, no de los crates. `encode`
//! resolvía la marca con un `match` cuyas dos ramas devolvían `fbf`, y
//! `render` ni siquiera miraba el guion. Una prueba que llamara a
//! `theme_by_name` habría pasado con ese código intacto.

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
    Command::new(env!("CARGO_BIN_EXE_guion"))
        .args(args)
        .current_dir(raiz())
        .output()
        .expect("correr guion")
}

fn tmp(nombre: &str) -> PathBuf {
    let d = std::env::temp_dir().join("guion-r0008").join(nombre);
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Copia el guion de ejemplo cambiándole la marca.
fn con_marca(marca: &str, nombre: &str) -> PathBuf {
    let src = std::fs::read_to_string(raiz().join("templates/reel-09-biceps.screenplay.toml"))
        .expect("el guion de ejemplo");
    let mut out = String::new();
    for l in src.lines() {
        if l.trim_start().starts_with("theme") {
            out.push_str(&format!("theme  = \"{marca}\"\n"));
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    let d = tmp(nombre);
    std::fs::create_dir_all(&d).expect("mkdir");
    let f = d.join("guion.toml");
    std::fs::write(&f, out).expect("escribir");
    f
}

/// Lee un cuadro PPM binario: devuelve (inicio de los pixeles, bytes).
fn ppm(p: &Path) -> (usize, Vec<u8>) {
    let b = std::fs::read(p).expect("el cuadro");
    let (mut saltos, mut i) = (0, 0);
    while saltos < 3 {
        if b[i] == b'\n' {
            saltos += 1;
        }
        i += 1;
    }
    (i, b)
}

fn render(marca: &str, nombre: &str) -> PathBuf {
    let guion_toml = con_marca(marca, nombre);
    let dir = tmp(&format!("{nombre}-out"));
    let out = guion(&[
        "render",
        guion_toml.to_str().expect("utf8"),
        "--out",
        dir.to_str().expect("utf8"),
    ]);
    assert!(
        out.status.success(),
        "render con marca `{marca}`: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    dir
}

/// AC1. La MISMA pieza bajo dos marcas no puede dar el mismo cuadro. Es
/// la prueba mínima de que la marca llega al render, y es exactamente la
/// que faltaba: hoy se publicó por poco una pieza renderizada con la
/// identidad equivocada y ninguna de las 41 comprobaciones lo dijo,
/// porque ninguna preguntaba por la marca.
#[test]
fn la_misma_pieza_bajo_dos_marcas_no_da_el_mismo_cuadro() {
    let a = render("fbf", "fbf").join("frame_00030.ppm");
    let b = render("pista", "pista").join("frame_00030.ppm");
    let (_, ba) = ppm(&a);
    let (_, bb) = ppm(&b);
    assert_ne!(ba, bb, "las dos marcas dieron el cuadro idéntico");
}

/// AC2. Y no difieren de cualquier modo: `pista` tiene la tinta CLARA y
/// `fbf` la oscura, así que sobre los pixeles que cambian, los de
/// `pista` tienen que ser netamente más claros. Un cambio cualquiera
/// (ruido, un sello, una fecha) también haría fallar AC1; esto fija la
/// DIRECCIÓN del cambio.
#[test]
fn la_tinta_de_pista_es_la_clara_y_se_nota_en_el_cuadro() {
    let (ia, ba) = ppm(&render("fbf", "fbf2").join("frame_00030.ppm"));
    let (ib, bb) = ppm(&render("pista", "pista2").join("frame_00030.ppm"));
    let (mut luz_a, mut luz_b, mut n) = (0u64, 0u64, 0u64);
    for p in 0..(1080 * 1920) {
        let (oa, ob) = (ia + p * 3, ib + p * 3);
        if ba[oa..oa + 3] != bb[ob..ob + 3] {
            n += 1;
            luz_a += ba[oa..oa + 3].iter().map(|v| *v as u64).sum::<u64>();
            luz_b += bb[ob..ob + 3].iter().map(|v| *v as u64).sum::<u64>();
        }
    }
    assert!(n > 1000, "casi no cambió nada: {n} pixeles");
    let (ma, mb) = (luz_a as f64 / n as f64, luz_b as f64 / n as f64);
    println!("sobre {n} pixeles distintos: fbf {ma:.1}  pista {mb:.1}");
    assert!(
        mb > ma * 1.5,
        "pista no salió más clara que fbf: {mb:.1} contra {ma:.1}"
    );
}

/// AC3. Una marca que no existe NO se renderiza con la de por omisión:
/// el proceso falla, nombra las marcas que sí hay, y no deja cuadros a
/// medias. Sin esta, AC1 y AC2 no sirven de nada — el fallo que de
/// verdad duele no es «salió feo», es «salió entero y con la identidad
/// de otro cliente», que se ve terminado.
#[test]
fn una_marca_que_no_existe_no_renderiza_nada() {
    let guion_toml = con_marca("pizta", "mala");
    let dir = tmp("mala-out");
    let out = guion(&[
        "render",
        guion_toml.to_str().expect("utf8"),
        "--out",
        dir.to_str().expect("utf8"),
    ]);
    assert!(!out.status.success(), "debería fallar y no falló");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("pizta"), "no dice qué se pidió: {err}");
    assert!(err.contains("fbf") && err.contains("pista"), "{err}");
    assert!(
        !dir.join("frame_00000.ppm").exists(),
        "escribió cuadros con una marca que no existe"
    );
}
