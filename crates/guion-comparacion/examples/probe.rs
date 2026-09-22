fn main() {
    let t = std::fs::read_to_string("guiones/reel40-gluteo.toml").expect("leer");
    match guion_comparacion::guion::cargar(&t) {
        Ok(c) => {
            println!(
                "CARGA: {}  ({} opciones, {} criterios)",
                c.titulo,
                c.opciones.len(),
                c.criterios.len()
            );
            for r in c.renglones() {
                println!("   {:?}", r);
            }
        }
        Err(e) => println!("FALLA: {e}"),
    }
}
