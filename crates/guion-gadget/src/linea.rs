//! The timeline (R-0015 AC4): clips and animations anchored to the voice,
//! never to a typed time.
//!
//! An anchor is a name and an optional offset: `inicio`, `fin`, a voice
//! line's id (when it starts: `gancho`) or its end (`gancho.fin`), each
//! `± seconds` (`cierre-0.3`). When the voice is re-synthesised and a line
//! moves, everything anchored to it moves with it; a renamed line fails
//! loudly, naming the anchor and the names there are. A bare number is
//! refused: that is the hand-typed time this model exists to remove.

use guion_reel::ease::{ease_out_cubic, progress};
use guion_reel::Narration;

/// The named instants of a post: what anchors resolve against.
#[derive(Debug, Clone, PartialEq)]
pub struct Tiempos {
    nombres: Vec<(String, f64)>,
    pub duracion: f64,
}

impl Tiempos {
    /// `inicio`, `fin` and, with a voice, each line's `<id>` and `<id>.fin`.
    pub fn de_voz(voz: Option<&Narration>, duracion: f64) -> Tiempos {
        let mut nombres = vec![("inicio".to_string(), 0.0), ("fin".to_string(), duracion)];
        for l in voz.map(|v| v.lines.as_slice()).unwrap_or_default() {
            nombres.push((l.id.clone(), l.t0));
            nombres.push((format!("{}.fin", l.id), l.t1));
        }
        Tiempos { nombres, duracion }
    }

    /// More named instants: the events of `eventos.json` (a beat, a
    /// gadget's state), by their names. A name the voice already has keeps
    /// the voice's instant.
    pub fn con_eventos(mut self, eventos: &[(String, f64)]) -> Tiempos {
        for (n, t) in eventos {
            if self.buscar(n).is_none() {
                self.nombres.push((n.clone(), *t));
            }
        }
        self
    }

    /// The names and their instants, in order (the studio lists them).
    pub fn nombres(&self) -> &[(String, f64)] {
        &self.nombres
    }

    fn buscar(&self, n: &str) -> Option<f64> {
        self.nombres.iter().find(|(m, _)| m == n).map(|(_, t)| *t)
    }

    /// The instant an anchor names; the error names the anchor and the
    /// names there are.
    pub fn resolver(&self, ancla: &str) -> Result<f64, String> {
        let s = ancla.trim();
        if let Some(t) = self.buscar(s) {
            return Ok(t);
        }
        // the offset is the last + or − whose prefix is a name
        for (i, c) in s.char_indices().rev() {
            if i == 0 || !(c == '+' || c == '-') {
                continue;
            }
            let (nombre, resto) = (s[..i].trim(), s[i..].replace(' ', ""));
            if let (Some(t), Ok(d)) = (self.buscar(nombre), resto.parse::<f64>()) {
                if d.is_finite() {
                    return Ok(t + d);
                }
            }
        }
        if s.parse::<f64>().is_ok() {
            return Err(format!(
                "ancla «{s}»: a time is anchored to the voice (a line id, inicio or fin, ± seconds), never typed"
            ));
        }
        let hay: Vec<&str> = self.nombres.iter().map(|(n, _)| n.as_str()).collect();
        Err(format!(
            "ancla «{s}» names no instant of this post; there are: {}",
            hay.join(", ")
        ))
    }
}

/// How an animation moves from `de` to `a`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curva {
    Lineal,
    /// Smoothstep: starts and ends at rest (a repetition).
    Suave,
    /// Cubic ease-out: the house's entrance.
    Sale,
}

impl Curva {
    pub const TODAS: [Curva; 3] = [Curva::Lineal, Curva::Suave, Curva::Sale];

    pub fn nombre(self) -> &'static str {
        match self {
            Curva::Lineal => "lineal",
            Curva::Suave => "suave",
            Curva::Sale => "sale",
        }
    }

    pub fn de(s: &str) -> Result<Curva, String> {
        Curva::TODAS
            .into_iter()
            .find(|c| c.nombre() == s)
            .ok_or_else(|| format!("curva «{s}»: one of lineal, suave, sale"))
    }

    /// The eased progress of `u` ∈ [0, 1].
    pub fn en(self, u: f64) -> f64 {
        match self {
            Curva::Lineal => u,
            Curva::Suave => u * u * (3.0 - 2.0 * u),
            Curva::Sale => ease_out_cubic(u),
        }
    }
}

/// An animation resolved to instants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tramo {
    pub de: f64,
    pub a: f64,
    pub t0: f64,
    pub t1: f64,
    pub curva: Curva,
}

impl Tramo {
    /// The value at `t`: `de` before `t0`, `a` after `t1`.
    pub fn en(&self, t: f64) -> f64 {
        if t <= self.t0 {
            return self.de;
        }
        if t >= self.t1 {
            return self.a;
        }
        self.de + (self.a - self.de) * self.curva.en(progress(t, self.t0, self.t1))
    }
}

/// A param's value at `t` under its animations, in file order: the first
/// one governs from the start of the clip (holding its `de` until it
/// starts); each later one takes over once it starts. Without animations,
/// the static value.
pub fn animar(estatico: f64, tramos: &[Tramo], t: f64) -> f64 {
    let mut v = estatico;
    for (k, tr) in tramos.iter().enumerate() {
        if k == 0 || t >= tr.t0 {
            v = tr.en(t);
        }
    }
    v
}

/// A clip's opacity at `t`: it fades in over `fundido` s from `t0` (unless
/// it starts the reel), out over the 0.25 s before `t1` (unless it lasts to
/// the end), and is 0 outside `[t0, t1]`.
pub fn opacidad(t: f64, t0: f64, t1: f64, fundido: f64, duracion: f64) -> f64 {
    if t < t0 || t > t1 {
        return 0.0;
    }
    let entra = if t0 <= 0.0 || fundido <= 0.0 {
        1.0
    } else {
        ease_out_cubic(progress(t, t0, t0 + fundido))
    };
    let sale = if t1 >= duracion {
        1.0
    } else {
        1.0 - progress(t, t1 - 0.25, t1)
    };
    entra * sale
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voz() -> Narration {
        Narration::from_json(
            r#"{"duracion_voz": 6.0, "lineas": [
                {"id": "gancho", "texto": "a", "subtitulo": "a", "t0": 0.5, "t1": 2.5},
                {"id": "cierre", "texto": "b", "subtitulo": "b", "t0": 3.0, "t1": 6.0}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn las_anclas_son_de_la_voz() {
        let v = voz();
        let t = Tiempos::de_voz(Some(&v), 8.0);
        assert_eq!(t.resolver("gancho").unwrap(), 0.5);
        assert_eq!(t.resolver("gancho.fin").unwrap(), 2.5);
        assert!((t.resolver("cierre-0.3").unwrap() - 2.7).abs() < 1e-12);
        assert_eq!(t.resolver("cierre + 1").unwrap(), 4.0);
        assert_eq!(t.resolver("fin").unwrap(), 8.0);
        let e = t.resolver("3.5").unwrap_err();
        assert!(e.contains("never typed"), "{e}");
        let e = t.resolver("gancho2").unwrap_err();
        assert!(e.contains("gancho.fin") && e.contains("cierre"), "{e}");
    }

    #[test]
    fn los_eventos_tambien_son_anclas() {
        // R-0016: a chapter may start on a beat of eventos.json
        let v = voz();
        let ev = [("beat-2".to_string(), 3.2), ("gancho".to_string(), 9.0)];
        let t = Tiempos::de_voz(Some(&v), 8.0).con_eventos(&ev);
        assert_eq!(t.resolver("beat-2").unwrap(), 3.2);
        assert!((t.resolver("beat-2-0.2").unwrap() - 3.0).abs() < 1e-12);
        assert_eq!(
            t.resolver("gancho").unwrap(),
            0.5,
            "the voice's name keeps the voice's instant"
        );
    }

    #[test]
    fn la_primera_animacion_manda_desde_el_principio() {
        let sube = Tramo {
            de: 0.0,
            a: 90.0,
            t0: 1.0,
            t1: 2.0,
            curva: Curva::Lineal,
        };
        let baja = Tramo {
            de: 90.0,
            a: 0.0,
            t0: 3.0,
            t1: 4.0,
            curva: Curva::Suave,
        };
        let a = [sube, baja];
        assert_eq!(animar(45.0, &[], 1.5), 45.0);
        assert_eq!(animar(45.0, &a, 0.0), 0.0);
        assert_eq!(animar(45.0, &a, 1.5), 45.0);
        assert_eq!(animar(45.0, &a, 2.5), 90.0);
        assert_eq!(animar(45.0, &a, 3.5), 45.0);
        assert_eq!(animar(45.0, &a, 9.0), 0.0);
    }

    #[test]
    fn la_opacidad_de_un_clip() {
        assert_eq!(opacidad(0.0, 0.0, 10.0, 0.3, 10.0), 1.0);
        assert_eq!(opacidad(1.0, 1.0, 10.0, 0.3, 10.0), 0.0);
        assert_eq!(opacidad(1.3, 1.0, 10.0, 0.3, 10.0), 1.0);
        assert_eq!(opacidad(5.0, 1.0, 5.0, 0.3, 10.0), 0.0);
        assert_eq!(opacidad(5.5, 1.0, 5.0, 0.3, 10.0), 0.0);
        assert_eq!(opacidad(2.0, 1.0, 5.0, 0.0, 10.0), 1.0);
    }
}
