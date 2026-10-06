//! El camino del dialecto **alto**: `[comparacion]`.
//!
//! El lector (`guion_comparacion::guion::cargar`) y el dibujo
//! (`guion_render::escena_grafica`) existían desde antes y corrían en
//! pruebas y ejemplos. Lo que faltaba era el cable: que el CLI supiera
//! que ese dialecto existe. Sin él, `guiones/reel40-gluteo.toml` —el
//! primer guion declarativo del proyecto, con sus números congelados en
//! `fixtures/dorados.json`— fallaba con «unknown field `comparacion`».
//!
//! # Lo que este camino todavía NO hace
//!
//! Da la **gráfica**, no el molde entero. No hay figura, ni paneles, ni
//! tarjeta final: eso vive hoy en la fábrica de Python y su port es
//! trabajo aparte. Se dice en voz alta al renderizar, porque un comando
//! que escribe cuadros y calla lo que falta deja creer que ya está.

use std::path::Path;
use std::process::ExitCode;

use guion_comparacion::{guion, Comparacion, Veredicto};
use guion_render::{escena_grafica, PAPEL, VISTA};
use motoreel::{FrameSink, PpmSink};

/// Lee el guion y devuelve la comparación, o imprime por qué no pudo.
pub fn cargar(ruta: &Path) -> Result<Comparacion, ExitCode> {
    let texto = match std::fs::read_to_string(ruta) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}: {e}", ruta.display());
            return Err(ExitCode::from(1));
        }
    };
    guion::cargar(&texto).map_err(|e| {
        eprintln!("el guion no carga: {e}");
        ExitCode::from(1)
    })
}

/// `guion check` sobre un guion de comparación.
///
/// Imprime los renglones con su veredicto, que es lo que el creador
/// quiere ver sin esperar un render: si su comparación dice algo o no.
pub fn check(ruta: &Path) -> ExitCode {
    let c = match cargar(ruta) {
        Ok(c) => c,
        Err(code) => return code,
    };
    println!("ok: {}  ·  objetivo: {}", c.titulo, c.objetivo);
    let nombres: Vec<&str> = c.opciones.iter().map(|o| o.nombre.as_str()).collect();
    println!("   opciones: {}", nombres.join("  vs  "));
    for r in c.renglones() {
        let vals: Vec<String> = r.valores.iter().map(|v| format!("{v:.1}")).collect();
        println!(
            "   {:<28} {:>20}   {}",
            r.etiqueta,
            vals.join("  "),
            match r.veredicto {
                Veredicto::Gana(i) => format!("gana {}", nombres.get(i).unwrap_or(&"?")),
                Veredicto::Empatan => "empatan".to_string(),
                Veredicto::SinGanador => "sin ganador".to_string(),
            }
        );
    }
    if c.opciones.len() >= 2 {
        if let Some(s) = c.cruce(0, 1) {
            println!(
                "   las curvas se cruzan al {:.1}% de la repetición",
                s * 100.0
            );
        }
    }
    ExitCode::SUCCESS
}

/// `guion render` sobre un guion de comparación: la gráfica, trazándose.
///
/// La curva se dibuja **con** el movimiento y no aparece hecha, que es lo
/// que la hace leerse como una medición y no como un adorno.
pub fn render(ruta: &Path, dir: &Path, cuadros: usize) -> ExitCode {
    let c = match cargar(ruta) {
        Ok(c) => c,
        Err(code) => return code,
    };
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("{}: {e}", dir.display());
        return ExitCode::from(1);
    }
    // Un sink por corrida y un cuadro por índice: el ejemplo de
    // `guion-render` abría un directorio por cuadro, que sirve para
    // mirar uno y no para encodear la secuencia.
    let mut sink = match PpmSink::with_view(dir, (1080, 648), VISTA) {
        Ok(s) => s.with_background(PAPEL),
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    for k in 0..cuadros {
        let hasta = if cuadros <= 1 {
            1.0
        } else {
            k as f64 / (cuadros - 1) as f64
        };
        let escena = escena_grafica(&c, hasta, 200);
        if let Err(e) = sink.frame(k, &escena.eval(0.0)) {
            eprintln!("cuadro {k}: {e}");
            return ExitCode::from(1);
        }
    }
    println!("{cuadros} cuadros → {}", dir.display());
    println!("   {}", c.titulo);
    // Se dice lo que falta. Un comando que escribe cuadros y calla el
    // resto deja creer que el molde ya sale por aquí, y no sale.
    println!(
        "   (por ahora sólo la gráfica: sin figura, sin paneles y sin \
         tarjeta final)"
    );
    ExitCode::SUCCESS
}
