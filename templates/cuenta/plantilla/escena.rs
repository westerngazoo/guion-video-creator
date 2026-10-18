//! Reel «{{titulo}}» ({{cuenta}}), creado el {{fecha}} con `guion reel nuevo`
//! desde `plantillas/reel`: un esqueleto que compila, se renderiza y pasa la
//! compuerta (el título, el subtítulo y la tarjeta de cierre). Los beats los
//! escribe el autor.
//!
//! **La voz manda el reloj:** todos los instantes salen de `voz.json`
//! (`guion reel voice`); aquí solo se escribe cuánto dura cada movimiento.
//! Cada cuadro es una función pura del tiempo. Los colores, las caras y los
//! tamaños son los de `cuenta.toml` (las primitivas de `guion-marca`).
//!
//! Uso: `guion reel posts/{{slug}}`; el binario solo, con `--sheet` (la hoja
//! de contacto, `cuadros/hoja.png`) o `--frames 1.2,4.0` (cuadros sueltos).

use std::error::Error;
use std::path::Path;

use guion_marca::reel::ease::{ease_out_cubic, progress};
use guion_marca::reel::{Command, Narration, NoOverlay};
use guion_marca::{Marca, Prim2};

/// Lo que el reel sigue después de que acaba la voz: la tarjeta de cierre se
/// queda quieta (meta 2.5 s, mínimo 1.5).
const COLA: f64 = 2.3;

/// Todos los instantes del reel, sacados de la voz.
struct Tiempos {
    /// (inicio, fin) de cada beat: el gancho y el cierre (agrega los tuyos en medio).
    beats: Vec<(f64, f64)>,
    duracion: f64,
}

impl Tiempos {
    fn de(v: &Narration, fps: f64) -> Result<Tiempos, Box<dyn Error>> {
        // cada beat empieza 0.3 s antes de su primera línea
        let inicios = [0.0, v.span("cierre")?.0 - 0.3];
        let duracion = ((v.duration + COLA) * fps).round() / fps;
        let beats = inicios
            .iter()
            .enumerate()
            .map(|(i, &a)| (a, inicios.get(i + 1).copied().unwrap_or(duracion)))
            .collect();
        Ok(Tiempos { beats, duracion })
    }
}

/// La opacidad del beat `k` en `t`: entra en 0.3 s y sale en los 0.25 s
/// antes del siguiente.
fn opacidad(beats: &[(f64, f64)], t: f64, k: usize) -> f64 {
    let (a, b) = beats[k];
    if t < a || t > b {
        return 0.0;
    }
    let entra = if k == 0 { 1.0 } else { ease_out_cubic(progress(t, a, a + 0.3)) };
    let sale = if k + 1 == beats.len() { 1.0 } else { 1.0 - progress(t, b - 0.25, b) };
    entra * sale
}

fn cuadro(m: &Marca, voz: &Narration, k: &Tiempos, t: f64) -> (Vec<Prim2>, Vec<NoOverlay>) {
    let ultimo = k.beats.len() - 1;
    let mut v = Vec::new();

    // ===== 1 · gancho: la pregunta =====
    let o = opacidad(&k.beats, t, 0);
    if o > 0.0 {
        let entra = ease_out_cubic(progress(t, 0.1, 0.6));
        v.extend(m.titulo("{{titulo}}", o * entra));
    }

    // ===== los beats del autor van aquí: uno por idea, con su línea de voz =====

    // ===== último · cierre: la tarjeta de la cuenta =====
    if opacidad(&k.beats, t, ultimo) > 0.0 {
        v.extend(m.cierre(t - k.beats[ultimo].0));
    }

    // el subtítulo, encima de todo
    v.extend(m.subtitulo(voz, t));
    (v, Vec::new())
}

fn main() -> Result<(), Box<dyn Error>> {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let post = raiz.join("posts/{{slug}}");
    let m = Marca::cargar(&raiz)?;
    let voz = Narration::load(&post.join("voz.json"))?;
    let k = Tiempos::de(&voz, m.canvas.fps)?;

    // --- los asserts prueban algo (el proceso: docs/proceso-reel.md) ---
    m.comprobar_subtitulos(&voz)?;
    for w in k.beats.windows(2) {
        assert!(w[1].0 - w[0].0 >= 1.2, "un beat de menos de 1.2 s: {w:?}");
    }
    assert!(
        k.duracion >= voz.duration && k.duracion <= 90.0,
        "dura {} s",
        k.duracion
    );

    // el reloj: voz:<id>, beat-N y cierre, para el sonido (un solo reloj)
    let mut reloj = m.reloj(&voz)?;
    for (i, &(a, _)) in k.beats.iter().enumerate() {
        reloj = reloj.event(&format!("beat-{}", i + 1), a)?;
    }
    let reloj = reloj.event("cierre", k.beats[k.beats.len() - 1].0)?;

    let reel = m.reel("{{slug}}", post, k.duracion, reloj);
    let args: Vec<String> = std::env::args().skip(1).collect();
    for f in reel.run(&Command::from_args(&args)?, |t| cuadro(&m, &voz, &k, t))? {
        eprintln!("{}", f.display());
    }
    Ok(())
}
