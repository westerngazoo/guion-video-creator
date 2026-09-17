//! Dibuja la gráfica de un guion. `cargo run -p guion-render --example grafica -- <guion.toml> <dir>`
//!
//! Prueba el camino entero: TOML → mecanica → Comparacion → motoreel →
//! cuadros. Ninguna curva está escrita a mano en ningún lado.
use guion_comparacion::guion;
use guion_render::{escena_grafica, PAPEL, VISTA};
use motoreel::PpmSink;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let toml = args.get(1).cloned().unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../guiones/reel40-gluteo.toml"
        )
        .to_string()
    });
    let dir = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "out/grafica".to_string());

    let c = match guion::cargar(&std::fs::read_to_string(&toml)?) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("el guion no carga: {e}");
            std::process::exit(1);
        }
    };
    std::fs::create_dir_all(&dir)?;

    // una repetición: la curva se traza CON el movimiento
    let cuadros = 60;
    for k in 0..=cuadros {
        let hasta = k as f64 / cuadros as f64;
        let escena = escena_grafica(&c, hasta, 200);
        let sub = format!("{dir}/f{k:04}");
        std::fs::create_dir_all(&sub)?;
        let mut sink = PpmSink::with_view(&sub, (1080, 648), VISTA)?.with_background(PAPEL);
        escena.render(1.0, &mut sink)?;
    }
    println!("{} · {} cuadros en {dir}", c.titulo, cuadros + 1);
    if let Some(s) = c.cruce(0, 1) {
        println!("  (las curvas se cruzan al {:.1}%)", s * 100.0);
    }
    Ok(())
}
