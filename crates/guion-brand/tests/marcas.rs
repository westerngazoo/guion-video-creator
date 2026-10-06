//! R-0008 — la segunda marca.
//!
//! El motor deja de tener UNA identidad adentro y pasa a tener una tabla
//! de marcas. Estas pruebas cuidan las dos cosas que hacen que eso no sea
//! sólo «otro archivo de colores»: que la marca pedida sea la que sale, y
//! que los colores de cada marca estén MEDIDOS contra su propio suelo.

use guion_brand::{theme_by_name, PaletteFile};

/// Luminancia relativa de WCAG 2.x. Se escribe aquí y no se importa
/// porque es la definición del estándar, no un detalle del motor: si
/// alguien la cambia, la prueba deja de significar lo que dice medir.
fn luminancia(c: [u8; 3]) -> f64 {
    let can = |v: u8| {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * can(c[0]) + 0.7152 * can(c[1]) + 0.0722 * can(c[2])
}

fn contraste(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (la, lb) = (luminancia(a), luminancia(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

const FBF: &str = include_str!("../../../themes/fbf/palette.toml");
const PISTA: &str = include_str!("../../../themes/pista/palette.toml");

/// AC1. `pista` existe y es OTRA marca: su papel es oscuro y su tinta
/// clara, o sea lo contrario de `fbf`. Una prueba que sólo comprobara
/// «resuelve sin error» habría pasado igual cuando las dos ramas del
/// `match` devolvían `fbf`.
#[test]
fn pista_es_otra_marca_no_la_de_siempre() {
    let fbf = PaletteFile::parse(FBF).expect("fbf carga");
    let pista = PaletteFile::parse(PISTA).expect("pista carga");

    assert!(
        luminancia(pista.colors.paper) < luminancia(pista.colors.ink),
        "pista tiene que ser papel oscuro con tinta clara"
    );
    assert!(
        luminancia(fbf.colors.paper) > luminancia(fbf.colors.ink),
        "fbf tiene que seguir siendo papel claro con tinta oscura"
    );
    assert_ne!(
        fbf.colors.accent, pista.colors.accent,
        "dos marcas con el mismo acento no son dos marcas"
    );
    assert!(
        theme_by_name("pista").is_ok() && theme_by_name("fbf").is_ok(),
        "las dos marcas embebidas se resuelven por nombre"
    );
}

/// AC2. Un nombre que no existe NO cae calladamente en la marca por
/// omisión: es un error que nombra las que sí existen. Éste es el
/// defecto que la fábrica vieja tenía y que dejó una pieza entera
/// renderizada con la identidad equivocada sin que nada se quejara.
#[test]
fn una_marca_que_no_existe_no_se_adivina() {
    let e = theme_by_name("pizta").expect_err("debería fallar");
    assert!(e.contains("pizta"), "no dice qué se pidió: {e}");
    assert!(e.contains("fbf"), "no ofrece las que hay: {e}");
    assert!(e.contains("pista"), "no ofrece las que hay: {e}");
}

/// AC7. El acento de `pista` es FRÍO, y eso no es gusto: la escala de
/// calor recorre verde→amarillo→naranja→rojo, así que un acento cálido
/// competiría con «esto está caliente» en vez de sumarle. La prueba lo
/// fija en azul: el canal azul manda sobre el rojo.
#[test]
fn el_acento_de_pista_es_frio_porque_el_calor_ya_ocupa_lo_calido() {
    let p = PaletteFile::parse(PISTA).expect("pista carga");
    for (que, c) in [("accent", p.colors.accent), ("accent2", p.colors.accent2)] {
        assert!(
            c[2] > c[0],
            "{que} = {c:?} no es frío; el rojo le gana al azul"
        );
    }
    // Y el extremo caliente de la rampa sigue siendo cálido, si no la
    // escala dejaría de significar nada.
    let caliente = p.heat.last().expect("la rampa tiene extremos").rgb;
    assert!(caliente[0] > caliente[2], "el tope del calor no es cálido");
}

/// AC4. Cada tinta se mide contra la superficie sobre la que DE VERDAD
/// se pinta, no contra «el papel» por costumbre.
///
/// El primer intento de esta prueba midió todo contra `paper` y reprobó
/// tres colores de `fbf` — la marca que lleva 47 piezas publicadas. No
/// eran defectos: era el par equivocado. El segundo intento supuso que
/// toda marca tiene dos superficies, una clara y una oscura, y que
/// `accent` va sobre la clara; en `pista` eso midió cian contra el color
/// de TEXTO (1.54:1) porque en una marca de suelo oscuro `ink` no es
/// fondo de nada. Una prueba que mide un par que no ocurre es peor que
/// no tenerla: el primero que la vea reprobar le va a bajar el umbral.
///
/// Las superficies son dos y se llaman por su nombre: `paper` (la
/// página) y `bar` (el cintillo del pie). `ink` nunca es fondo.
#[test]
fn cada_tinta_contra_la_superficie_en_la_que_se_pinta() {
    let mut flojos = Vec::new();
    for (marca, src) in [("fbf", FBF), ("pista", PISTA)] {
        let p = PaletteFile::parse(src).expect("carga");
        // `ink` sobre `paper` es el texto largo de la pieza: 7:1.
        // `accent` va sobre la página — 4.1:1, que es el umbral que la
        // marca se puso a sí misma y documenta en fabrica.py.
        // `accent2` va sobre el cintillo: 4.5:1.
        for (que, c, fondo, min) in [
            ("ink", p.colors.ink, p.colors.paper, 7.0),
            ("accent", p.colors.accent, p.colors.paper, 4.1),
            ("accent2", p.colors.accent2, p.colors.bar, 4.5),
        ] {
            let r = contraste(c, fondo);
            println!("{marca:>5} {que:>8} {c:?} sobre {fondo:?} = {r:.2}:1");
            if r < min {
                flojos.push(format!("{marca}/{que}: {r:.2}:1 < {min}:1"));
            }
        }
    }
    assert!(flojos.is_empty(), "colores que no se leen: {flojos:#?}");
}

/// AC5. La rampa de calor tiene que LEERSE como escala: lo que la hace
/// una escala no es el contraste contra el fondo — los tonos van con
/// contorno y dentro de la tira de leyenda, nunca solos sobre el papel —
/// sino que cada tono se distinga de su vecino.
///
/// El piso es 1.5:1 porque es lo que logra `fbf`, que ya publica. La
/// regla es un trinquete declarado: **ninguna marca nueva puede leerse
/// peor que la que ya funciona**. No es un umbral acomodado para que
/// pase; es el listón puesto donde está la vara actual.
///
/// El contraste contra el papel se IMPRIME y no se afirma: es el número
/// que hará falta el día que se codifique la regla del contorno, y
/// dejarlo a la vista es más honesto que afirmar un par que hoy no
/// gobierna nada.
#[test]
fn la_rampa_de_calor_se_lee_como_escala() {
    let mut flojos = Vec::new();
    for (marca, src) in [("fbf", FBF), ("pista", PISTA)] {
        let p = PaletteFile::parse(src).expect("carga");
        assert!(p.heat.len() >= 3, "{marca}: una rampa de dos no es escala");
        for (i, s) in p.heat.iter().enumerate() {
            println!(
                "{marca:>5} heat[{i}] t={:.2} {:?} · vs papel {:.2}:1",
                s.t,
                s.rgb,
                contraste(s.rgb, p.colors.paper)
            );
        }
        for w in p.heat.windows(2) {
            let r = contraste(w[0].rgb, w[1].rgb);
            if r < 1.5 {
                flojos.push(format!(
                    "{marca}: {:?} y {:?} se confunden ({r:.2}:1 < 1.5:1)",
                    w[0].rgb, w[1].rgb
                ));
            }
        }
        // Y sobre suelo oscuro hay una exigencia extra que sobre crema no
        // aplica: un tono apagado no se «lee tenue», desaparece.
        if luminancia(p.colors.paper) < luminancia(p.colors.ink) {
            for s in &p.heat {
                let r = contraste(s.rgb, p.colors.paper);
                if r < 3.0 {
                    flojos.push(format!(
                        "{marca}: {:?} se hunde en el fondo ({r:.2}:1 < 3:1)",
                        s.rgb
                    ));
                }
            }
        }
    }
    assert!(flojos.is_empty(), "la escala no se lee: {flojos:#?}");
}

/// AC6. `skin` es el mismo en las dos marcas. La piel es anatomía, no
/// identidad: un muñeco que cambia de color con el cliente estaría
/// diciendo algo que no es cierto.
#[test]
fn la_piel_no_es_identidad_de_marca() {
    let fbf = PaletteFile::parse(FBF).expect("fbf carga");
    let pista = PaletteFile::parse(PISTA).expect("pista carga");
    assert_eq!(fbf.colors.skin, pista.colors.skin);
}
