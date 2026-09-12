//! Un reel es una **comparación**, y de ella se deriva todo lo demás.
//!
//! Ésta es la capa que faltaba (ver `docs/DISENO.md`). Antes, cada reel
//! era un archivo: `reel37.py`, `reel38.py`, `reel39.rs`, cada uno con sus
//! cuatrocientas líneas repitiendo la misma forma. Aquí la forma se
//! escribe **una vez** y el creador sólo declara el caso.
//!
//! La física no vive aquí. Vive en `mecanica`, que reduce un ejercicio a
//! una función —cuánto resiste la carga en cada punto del movimiento— y
//! ya trae las algoritmias encima: [`mecanica::peak`],
//! [`mecanica::work_over_range`], [`mecanica::tau_at_progress`],
//! [`mecanica::crossing_progress`]. Este crate no calcula un solo torque.
//!
//! Lo que sí vive aquí es la **política editorial**: qué se compara, en
//! qué sentido, y cuándo dos cosas se declaran iguales. Eso es una
//! decisión del creador, no de la física, y por eso es un dato y no un
//! `if` escondido dentro de un reel.

#![forbid(unsafe_code)]

pub mod catalogo;
pub mod guion;

use mecanica::{crossing_progress, peak, tau_at_progress, work_over_range, Lift};

/// Hacia dónde es "mejor" en un criterio.
///
/// `Informativo` existe porque la mayoría de los hallazgos buenos no
/// tienen ganador: en el press de hombro el militar pica más arriba y el
/// Smith es parejo, y coronar a uno sería inventar una conclusión que los
/// números no dan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sentido {
    MayorEsMejor,
    MenorEsMejor,
    Informativo,
}

/// Qué se mide en un renglón de la comparación.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Medida {
    /// El torque máximo de la repetición, N·m.
    Pico,
    /// El trabajo por repetición, J.
    Trabajo,
    /// El torque en un punto del recorrido (0 inicio, 1 cierre), N·m.
    EnProgreso(f64),
}

/// Un renglón de la comparación: qué se mide y cómo se juzga.
#[derive(Clone, Debug)]
pub struct Criterio {
    pub etiqueta: String,
    pub medida: Medida,
    pub sentido: Sentido,
    /// Diferencia relativa por debajo de la cual se declaran iguales.
    /// `0.15` es "iguales si difieren menos del 15%".
    pub empate_si: f64,
}

/// El fallo de un renglón.
///
/// `Empatan` y `SinGanador` son distintos a propósito, y la diferencia
/// llega hasta la pantalla. Un renglón informativo con 359 J contra 290 J
/// **no** puede rotularse "IGUALES": difieren un 19% y eso sería una
/// afirmación falsa, del tipo exacto que las compuertas existen para
/// atrapar. Lo que dice es que no coronamos a un ganador ahí.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Veredicto {
    /// Gana esa opción en ese renglón.
    Gana(usize),
    /// Los valores están dentro de `empate_si`: son el mismo número.
    Empatan,
    /// El criterio es informativo: hay diferencia, pero no es un ganador.
    SinGanador,
}

/// Una opción comparada: su nombre y su ejercicio.
pub struct Opcion {
    pub nombre: String,
    pub lift: Box<dyn Lift>,
}

/// Lo que un reel es.
pub struct Comparacion {
    pub titulo: String,
    /// La articulación que el ejercicio ENTRENA. Es la que va viva en
    /// pantalla y la que tiene que moverse con la repetición.
    pub objetivo: String,
    pub opciones: Vec<Opcion>,
    pub criterios: Vec<Criterio>,
}

/// Un renglón ya resuelto: los valores y quién gana.
#[derive(Clone, Debug)]
pub struct Renglon {
    pub etiqueta: String,
    pub valores: Vec<f64>,
    pub veredicto: Veredicto,
}

impl Comparacion {
    /// El valor de una medida para una opción.
    #[must_use]
    pub fn valor(&self, i: usize, medida: Medida) -> f64 {
        let l = self.opciones[i].lift.as_ref();
        match medida {
            Medida::Pico => peak(l).1,
            Medida::Trabajo => work_over_range(l),
            Medida::EnProgreso(s) => tau_at_progress(l, s),
        }
    }

    /// Quién gana un criterio.
    ///
    /// Con más de dos opciones el empate se juzga contra el extremo: si
    /// el mejor y el peor difieren menos de `empate_si`, no hay ganador.
    /// Coronar al mejor por un 3% sería presentar ruido como hallazgo.
    #[must_use]
    pub fn veredicto(&self, c: &Criterio) -> Veredicto {
        let v: Vec<f64> = (0..self.opciones.len())
            .map(|i| self.valor(i, c.medida))
            .collect();
        let (mut mejor, mut peor) = (0usize, 0usize);
        for (i, x) in v.iter().enumerate() {
            let gana = match c.sentido {
                Sentido::MenorEsMejor => *x < v[mejor],
                _ => *x > v[mejor],
            };
            if gana {
                mejor = i;
            }
            let pierde = match c.sentido {
                Sentido::MenorEsMejor => *x > v[peor],
                _ => *x < v[peor],
            };
            if pierde {
                peor = i;
            }
        }
        let escala = v[mejor].abs().max(v[peor].abs()).max(f64::EPSILON);
        if (v[mejor] - v[peor]).abs() / escala < c.empate_si {
            return Veredicto::Empatan;
        }
        if c.sentido == Sentido::Informativo {
            return Veredicto::SinGanador;
        }
        Veredicto::Gana(mejor)
    }

    /// La tabla entera: lo que va en la tarjeta final del reel.
    #[must_use]
    pub fn renglones(&self) -> Vec<Renglon> {
        self.criterios
            .iter()
            .map(|c| Renglon {
                etiqueta: c.etiqueta.clone(),
                valores: (0..self.opciones.len())
                    .map(|i| self.valor(i, c.medida))
                    .collect(),
                veredicto: self.veredicto(c),
            })
            .collect()
    }

    /// Dónde se cruzan dos opciones en el eje compartido, si se cruzan.
    ///
    /// El cruce suele ser el hallazgo: dos curvas que se cruzan no son
    /// competidoras, son las dos mitades de un mismo perfil.
    #[must_use]
    pub fn cruce(&self, i: usize, j: usize) -> Option<f64> {
        crossing_progress(
            self.opciones[i].lift.as_ref(),
            self.opciones[j].lift.as_ref(),
        )
    }

    /// La curva de una opción, muestreada en el eje compartido.
    #[must_use]
    pub fn curva(&self, i: usize, n: usize) -> Vec<(f64, f64)> {
        let l = self.opciones[i].lift.as_ref();
        (0..=n)
            .map(|k| {
                let s = k as f64 / n as f64;
                (s, tau_at_progress(l, s))
            })
            .collect()
    }

    /// Cuánto se mueve el torque del objetivo, como fracción de su pico.
    ///
    /// La compuerta del molde: un τ que no se mueve se lee fijo, y un
    /// número fijo en pantalla no cuenta nada aunque sea correcto.
    #[must_use]
    pub fn recorrido_relativo(&self, i: usize, n: usize) -> f64 {
        let v: Vec<f64> = self.curva(i, n).into_iter().map(|(_, t)| t).collect();
        let (lo, hi) = v.iter().fold((f64::MAX, f64::MIN), |(a, b), x| {
            (a.min(*x), b.max(*x))
        });
        let pico = v.iter().fold(0.0_f64, |a, x| a.max(x.abs()));
        if pico == 0.0 {
            0.0
        } else {
            (hi - lo) / pico
        }
    }
}
