//! Imprime la comparación que sale de un guion. `cargo run --example tabla`
use guion_comparacion::{guion, Medida, Veredicto};

fn main() {
    // Toma el guion que le pases, o el de ejemplo si no le pasas ninguno.
    // La gracia es editar el TOML y volver a correr: nada que compilar.
    let ruta = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../guiones/reel40-gluteo.toml")
            .to_string()
    });
    let texto = match std::fs::read_to_string(&ruta) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("no pude leer {ruta}: {e}");
            std::process::exit(1);
        }
    };
    let c = match guion::cargar(&texto) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("el guion no carga: {e}");
            std::process::exit(1);
        }
    };
    println!("\n{}  ·  objetivo: {}\n", c.titulo, c.objetivo);
    print!("{:<26}", "");
    for o in &c.opciones {
        print!("{:>14}", o.nombre);
    }
    println!("\n{}", "-".repeat(26 + 14 * c.opciones.len()));
    for r in c.renglones() {
        print!("{:<26}", r.etiqueta);
        for (i, v) in r.valores.iter().enumerate() {
            let marca = if r.veredicto == Veredicto::Gana(i) { " *" } else { "  " };
            print!("{:>12.1}{marca}", v);
        }
        match r.veredicto {
            Veredicto::Empatan => print!("   iguales"),
            Veredicto::SinGanador => print!("   sin ganador"),
            Veredicto::Gana(_) => {}
        }
        println!();
    }
    println!("\n{:<26}", "τ al inicio / al cierre");
    for (i, o) in c.opciones.iter().enumerate() {
        println!("  {:<24}{:>8.1} -> {:>6.1} N·m   (se mueve {:.0}% del pico)",
                 o.nombre,
                 c.valor(i, Medida::EnProgreso(0.0)),
                 c.valor(i, Medida::EnProgreso(1.0)),
                 c.recorrido_relativo(i, 200) * 100.0);
    }
    if let Some(s) = c.cruce(0, 1) {
        println!("\n  las curvas se cruzan al {:.1}% de la subida", s * 100.0);
    }
}
