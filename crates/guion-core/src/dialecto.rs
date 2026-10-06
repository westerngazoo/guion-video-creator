//! Qué **clase** de guion es un archivo, antes de intentar leerlo.
//!
//! `guion` tiene dos dialectos y los dos son legítimos:
//!
//! - **[`Dialecto::Escena`]** — `[meta]`, `[camera]`, `[[object]]`,
//!   `[motion]`. Describe lo que se ve: es el nivel bajo, el que arma una
//!   escena de `motoreel` cuadro por cuadro.
//! - **[`Dialecto::Comparacion`]** — `[comparacion]`, `[[opcion]]`,
//!   `[[criterio]]`. Describe lo que se **compara**, y de ahí se deriva
//!   todo lo demás. Es el nivel alto, el que `docs/DISENO.md` pone como
//!   objetivo: *«que alguien sin programar pueda usar todo el framework»*.
//!
//! Hasta ahora el CLI sólo conocía el primero, así que
//! `guiones/reel40-gluteo.toml` —escrito en el segundo, y con sus números
//! congelados en `fixtures/dorados.json`— fallaba con
//! *«unknown field `comparacion`»*. El lector del alto existía y corría en
//! pruebas y ejemplos; lo que faltaba era que el CLI supiera cuál tenía
//! en las manos.
//!
//! # Por qué se detecta y no se adivina
//!
//! La alternativa barata es intentar un dialecto y, si falla, intentar el
//! otro. Eso rompe el §6 de `CLAUDE.md` —*«nombra la ubicación»*—: un
//! `[meta]` con una llave mal escrita acabaría reportando el error del
//! OTRO dialecto, que no dice nada útil. Aquí se mira una llave que
//! distingue, se elige el lector, y el error que sale es el del dialecto
//! que el autor de verdad estaba escribiendo.

use serde::Deserialize;

/// La clase de guion que trae un texto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialecto {
    /// Nivel bajo: describe la escena. Tabla `[meta]`.
    Escena,
    /// Nivel alto: describe la comparación. Tabla `[comparacion]`.
    Comparacion,
}

/// Sólo las dos llaves que distinguen. Todo lo demás se ignora a
/// propósito: esto decide **quién** lee el archivo, no si es válido.
#[derive(Debug, Deserialize)]
struct Sonda {
    #[serde(default)]
    meta: Option<toml::Value>,
    #[serde(default)]
    comparacion: Option<toml::Value>,
}

/// Por qué no se pudo decidir qué clase de guion es.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoSeSabe {
    /// El texto no es TOML.
    NoEsToml(String),
    /// No tiene ni `[meta]` ni `[comparacion]`.
    SinNinguna,
    /// Tiene las dos, y entonces no hay forma de saber cuál quiso decir.
    ConLasDos,
}

impl core::fmt::Display for NoSeSabe {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NoSeSabe::NoEsToml(e) => write!(f, "no es TOML: {e}"),
            NoSeSabe::SinNinguna => write!(
                f,
                "no tiene [meta] ni [comparacion], así que no se sabe qué \
                 clase de guion es. Un guion de escena empieza con [meta]; \
                 uno de comparación, con [comparacion]"
            ),
            NoSeSabe::ConLasDos => write!(
                f,
                "tiene [meta] Y [comparacion]: son dos dialectos distintos \
                 y no se pueden mezclar en un archivo. Parte el guion en dos"
            ),
        }
    }
}

impl std::error::Error for NoSeSabe {}

/// Qué clase de guion es `texto`.
///
/// # Errors
/// [`NoSeSabe`] cuando el texto no es TOML, no trae ninguna de las dos
/// tablas, o trae las dos.
pub fn dialecto(texto: &str) -> Result<Dialecto, NoSeSabe> {
    let s: Sonda = toml::from_str(texto).map_err(|e| NoSeSabe::NoEsToml(e.to_string()))?;
    match (s.meta.is_some(), s.comparacion.is_some()) {
        (true, false) => Ok(Dialecto::Escena),
        (false, true) => Ok(Dialecto::Comparacion),
        (true, true) => Err(NoSeSabe::ConLasDos),
        (false, false) => Err(NoSeSabe::SinNinguna),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_guion_de_escena_se_reconoce_por_meta() {
        let t = "[meta]\nslug = \"x\"\nfps = 30\n";
        assert_eq!(dialecto(t), Ok(Dialecto::Escena));
    }

    #[test]
    fn un_guion_de_comparacion_se_reconoce_por_comparacion() {
        let t = "[comparacion]\ntitulo = \"x\"\n";
        assert_eq!(dialecto(t), Ok(Dialecto::Comparacion));
    }

    /// La sonda ignora TODO lo demás: decide quién lee, no si es válido.
    /// Un `[meta]` incompleto sigue siendo un guion de escena, y tiene que
    /// llegar al lector de escena para que ÉSE dé el error bueno.
    #[test]
    fn un_guion_roto_sigue_teniendo_dialecto() {
        let t = "[meta]\nesto_no_existe = 1\n";
        assert_eq!(dialecto(t), Ok(Dialecto::Escena));
        let t = "[comparacion]\nni_esto = true\n\n[[opcion]]\nnombre = \"a\"\n";
        assert_eq!(dialecto(t), Ok(Dialecto::Comparacion));
    }

    #[test]
    fn sin_ninguna_de_las_dos_lo_dice_y_dice_cuáles_son() {
        let e = dialecto("[otra_cosa]\nx = 1\n").unwrap_err();
        assert_eq!(e, NoSeSabe::SinNinguna);
        let s = e.to_string();
        assert!(s.contains("[meta]"), "{s}");
        assert!(s.contains("[comparacion]"), "{s}");
    }

    /// Las dos a la vez es un error y no una preferencia. Elegir una
    /// callada sería exactamente el "silently ignored" que §6 prohíbe.
    #[test]
    fn con_las_dos_se_niega_en_vez_de_elegir() {
        let t = "[meta]\nslug = \"x\"\n\n[comparacion]\ntitulo = \"y\"\n";
        assert_eq!(dialecto(t), Err(NoSeSabe::ConLasDos));
    }

    #[test]
    fn lo_que_no_es_toml_lo_dice_y_no_entra_en_panico() {
        let e = dialecto("{{{ esto no es toml").unwrap_err();
        assert!(matches!(e, NoSeSabe::NoEsToml(_)), "{e:?}");
    }

    /// El archivo real que motivó todo esto.
    #[test]
    fn el_guion_del_reel40_es_de_comparacion() {
        let ruta = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../guiones/reel40-gluteo.toml"
        );
        let t = std::fs::read_to_string(ruta).expect("el guion del reel 40 existe");
        assert_eq!(dialecto(&t), Ok(Dialecto::Comparacion));
    }
}
