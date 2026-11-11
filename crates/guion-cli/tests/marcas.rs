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

/// El color que más se repite en un cuadro: la página.
fn pagina(dir: &Path) -> ([u8; 3], f64) {
    let (i, b) = ppm(&dir.join("frame_00030.ppm"));
    let mut cuenta: std::collections::HashMap<[u8; 3], u32> = std::collections::HashMap::new();
    let total = 1080 * 1920;
    for p in 0..total {
        let o = i + p * 3;
        *cuenta.entry([b[o], b[o + 1], b[o + 2]]).or_insert(0) += 1;
    }
    let (c, n) = cuenta
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .expect("el cuadro tiene pixeles");
    (c, f64::from(n) / total as f64)
}

/// AC2 (R-0008 OQ-1). El cuadro se limpia con el **papel de la marca**.
///
/// Antes de esto el fondo salía negro en las dos marcas y sólo cambiaban
/// los trazos, o sea que en `fbf` —papel crema— lo único que no llegaba
/// era la mitad de la pieza. Se mide el color que más se repite: es la
/// página, y tiene que ser exactamente el de la paleta.
#[test]
fn el_cuadro_se_limpia_con_el_papel_de_la_marca() {
    for (marca, nombre) in [("fbf", "fbf2"), ("pista", "pista2")] {
        let esperado = guion_brand::theme_by_name(marca).expect("la marca").paper();
        let (c, parte) = pagina(&render(marca, nombre));
        println!("{marca}: la página es {c:?} y ocupa {:.1}%", parte * 100.0);
        assert_eq!(
            c,
            [esperado.r, esperado.g, esperado.b],
            "{marca}: la página no es el papel de su marca"
        );
        assert!(
            parte > 0.5,
            "{marca}: el papel sólo ocupa {:.1}% del cuadro",
            parte * 100.0
        );
    }
}

/// AC9. Y las dos marcas son OPUESTAS, no variantes: en `fbf` la tinta
/// es más oscura que su página y en `pista` más clara. Fija la dirección
/// del cambio, que AC1 por sí sola no fija — un sello o una fecha
/// distinta también harían fallar AC1.
#[test]
fn una_marca_escribe_oscuro_sobre_claro_y_la_otra_al_reves() {
    for (marca, nombre, tinta_mas_clara) in [("fbf", "fbf3", false), ("pista", "pista3", true)] {
        let dir = render(marca, nombre);
        let (papel, _) = pagina(&dir);
        let (i, b) = ppm(&dir.join("frame_00030.ppm"));
        let luz = |c: &[u8]| c.iter().map(|v| u64::from(*v)).sum::<u64>();
        let (mut suma, mut n) = (0u64, 0u64);
        for p in 0..(1080 * 1920) {
            let o = i + p * 3;
            // Sólo lo que NO es página ni su versión tramada: el trazo.
            if b[o..o + 3] != papel && luz(&b[o..o + 3]).abs_diff(luz(&papel)) > 40 {
                suma += luz(&b[o..o + 3]);
                n += 1;
            }
        }
        assert!(n > 1000, "{marca}: casi no hay trazo ({n} pixeles)");
        let (mt, mp) = (suma as f64 / n as f64, luz(&papel) as f64);
        println!("{marca}: trazo {mt:.0}  ·  página {mp:.0}");
        assert_eq!(
            mt > mp,
            tinta_mas_clara,
            "{marca}: trazo {mt:.0} contra página {mp:.0}, no es lo esperado"
        );
    }
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
