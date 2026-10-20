//! R-0016 — the format of a gadget post: `formato` in `escena.toml` sets the
//! canvas the studio, its gadgets' `Contexto` and its render use; without it
//! nothing changes (the same text back, the same 9:16 canvas).

use std::path::PathBuf;

use guion_estudio::Estudio;
use guion_gadget::{Escena, Registro};
use guion_reel::{Canvas, Formato};

const ESCENA: &str = r#"version = 1
duracion = 2.0

[[gadget]]
id = "tau"
tipo = "medidor"

[gadget.params]
valor = { valor = 30.0, procedencia = "supuesto" }
rotulo = "τ"
"#;

fn post(name: &str, text: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-r16-estudio-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("escena.toml"), text).unwrap();
    d
}

#[test]
fn ac1_escena_formato_sets_the_studio_canvas_and_the_gadgets_context() {
    let vertical = Estudio::abrir(&post("v", ESCENA), Registro::integrados(), None).unwrap();
    assert_eq!(vertical.plantilla.canvas, Canvas::VERTICAL_REEL);
    assert_eq!(vertical.contexto().seguro, [60.0, 60.0, 1020.0, 1860.0]);

    let text = ESCENA.replace("version = 1\n", "version = 1\nformato = \"youtube\"\n");
    let p = post("h", &text);
    let e = Estudio::abrir(&p, Registro::integrados(), None).unwrap();
    assert_eq!(e.escena.formato, Some(Formato::Youtube));
    let cx = e.contexto();
    assert_eq!(cx.canvas, Canvas::HORIZONTAL_VIDEO);
    let b = Formato::Youtube.bandas();
    assert_eq!(
        cx.seguro,
        [b.safe_x[0], b.safe_y[0], b.safe_x[1], b.safe_y[1]]
    );
    // the frame is drawn at 1920 × 1080
    assert_eq!(e.cuadro_rgb(1.0).unwrap().len(), 1920 * 1080 * 3);
}

#[test]
fn ac3_a_scene_without_formato_saves_the_same_bytes() {
    let reg = Registro::integrados();
    let e = Escena::parse(ESCENA).unwrap();
    assert_eq!(e.formato, None);
    let saved = e.a_toml(&reg);
    assert!(!saved.contains("formato"), "{saved}");
    let y = Escena::parse(&ESCENA.replace("version = 1\n", "version = 1\nformato = \"short\"\n"))
        .unwrap();
    let again = y.a_toml(&reg);
    assert!(again.contains("formato = \"short\"\n"), "{again}");
    assert_eq!(Escena::parse(&again).unwrap(), y);
    let e = Escena::parse(&ESCENA.replace("version = 1\n", "version = 1\nformato = \"cine\"\n"))
        .unwrap_err();
    assert!(e.contains("cine") && e.contains("youtube"), "{e}");
}
