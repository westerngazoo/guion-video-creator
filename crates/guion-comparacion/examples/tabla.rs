//! Imprime la comparación que sale de un guion. `cargo run --example tabla`
use guion_comparacion::{guion, Medida, Veredicto};

fn main() {
    let ruta = concat!(env!("CARGO_MANIFEST_DIR"), "/../../guiones/reel40-gluteo.toml");
    let c = guion::cargar(&std::fs::read_to_string(ruta).unwrap()).unwrap();
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
