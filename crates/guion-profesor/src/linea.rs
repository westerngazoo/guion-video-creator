//! The class timeline: the voice sets the clock. Each beat lasts as long as
//! its spoken line (or its demonstration, if longer) plus the pause; chapters,
//! transcript cues and question stops all fall out of the same numbers, so
//! the sidecar the classroom reads cannot drift from the picture.

use serde::Serialize;

use crate::clase::{Beat, Clase, Escena, Pregunta};
use crate::fisica::{Frenado, Plano, Tiro};

/// Seconds between a beat's start and its demonstration's start.
pub const DEMO_LEAD: f64 = 0.6;
/// Seconds a finished demonstration stays on screen before the beat may end.
const DEMO_TAIL: f64 = 1.2;
/// How deep the cliff drops below the road, in metres.
pub const FONDO: f64 = 3.6;
#[derive(Debug, Clone, PartialEq)]
pub struct Tramo {
    pub t0: f64,
    /// When the spoken line ends.
    pub habla: f64,
    /// When the beat ends (pause included).
    pub t1: f64,
    pub capitulo: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Capitulo {
    pub t: f64,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cue {
    pub t0: f64,
    pub t1: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Parada {
    pub t: f64,
    pub chapter: usize,
    pub prompt: String,
    pub options: Vec<String>,
    pub answer: String,
    pub explain: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Linea {
    pub tramos: Vec<Tramo>,
    pub capitulos: Vec<Capitulo>,
    pub duracion: f64,
}

/// How long the demonstration of `beat` runs, in seconds (0 without one).
pub fn demo_duracion(escena: &Escena, beat: &Beat) -> f64 {
    let Some(d) = &beat.demo else { return 0.0 };
    let p = |k: &str| d.get(k).copied().unwrap_or(0.0);
    match escena {
        Escena::Frenado { v0, acantilado, .. } => Frenado {
            v0: *v0,
            a: p("a"),
            borde: *acantilado,
            fondo: FONDO,
        }
        .duracion(),
        Escena::Tiro { muralla, .. } => {
            let t = Tiro {
                th: p("th"),
                v0: p("v0"),
            };
            muralla
                .and_then(|m| t.choca(m.x, m.alto))
                .unwrap_or_else(|| t.vuelo())
        }
        Escena::Plano { largo, mu } => Plano {
            th: p("th"),
            mu: *mu,
            recorrido: largo - 1.0,
        }
        .duracion(),
    }
}

/// Lay the beats end to end. `habla[i]` is beat i's measured speech length.
pub fn plan(clase: &Clase, habla: &[f64]) -> Linea {
    assert_eq!(habla.len(), clase.beat.len(), "one speech length per beat");
    let pause = clase.narration.pause;
    let mut t = 0.0;
    let mut tramos = Vec::with_capacity(clase.beat.len());
    let mut capitulos = Vec::new();
    for (b, &h) in clase.beat.iter().zip(habla) {
        if let Some(title) = &b.capitulo {
            capitulos.push(Capitulo {
                t,
                title: title.clone(),
            });
        }
        let demo = demo_duracion(&clase.escena, b);
        let largo = if demo > 0.0 {
            h.max(DEMO_LEAD + demo + DEMO_TAIL)
        } else {
            h
        };
        let t1 = t + largo + pause;
        tramos.push(Tramo {
            t0: t,
            habla: t + h,
            t1,
            capitulo: capitulos.len() - 1,
        });
        t = t1;
    }
    Linea {
        tramos,
        capitulos,
        duracion: t,
    }
}

impl Linea {
    /// The beat on screen at `t` (the last one past the end).
    pub fn tramo_en(&self, t: f64) -> usize {
        self.tramos.iter().rposition(|tr| tr.t0 <= t).unwrap_or(0)
    }

    /// Transcript cues: each line split into sentences, timed by length.
    pub fn cues(&self, clase: &Clase) -> Vec<Cue> {
        let mut out = Vec::new();
        for (b, tr) in clase.beat.iter().zip(&self.tramos) {
            let frases = frases(&b.say);
            let total: usize = frases.iter().map(|f| f.chars().count()).sum();
            let mut t = tr.t0;
            for f in frases {
                let dt = (tr.habla - tr.t0) * f.chars().count() as f64 / total.max(1) as f64;
                out.push(Cue {
                    t0: round3(t),
                    t1: round3(t + dt),
                    text: f,
                });
                t += dt;
            }
        }
        out
    }

    /// Where the player stops to ask: right after the question is spoken.
    pub fn paradas(&self, clase: &Clase) -> Vec<Parada> {
        clase
            .beat
            .iter()
            .zip(&self.tramos)
            .filter_map(|(b, tr)| b.pregunta.as_ref().map(|q| parada(q, tr)))
            .collect()
    }
}

fn parada(q: &Pregunta, tr: &Tramo) -> Parada {
    Parada {
        t: round3(tr.habla),
        chapter: tr.capitulo,
        prompt: q.prompt.clone(),
        options: q.opciones.clone(),
        answer: q.correcta.clone(),
        explain: q.explica.clone(),
    }
}

/// Split a line after `.`, `?` or `!` (keeping the mark).
pub fn frases(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '?' | '!') {
            let f = cur.trim();
            if !f.is_empty() {
                out.push(f.to_string());
            }
            cur.clear();
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub fn round3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clase::parse;

    const SRC: &str = r#"
        [class]
        title = "Frenado"
        slug = "frenado"
        kicker = "Mecánica"
        [narration]
        pause = 0.5
        [escena]
        kind = "frenado"
        v0 = 8.0
        bandera = 16.0
        acantilado = 20.0
        [[beat]]
        capitulo = "Uno"
        say = "Hola. ¿Listo?"
        [[beat]]
        say = "Mira."
        demo = { a = 2.0 }
        [[beat]]
        capitulo = "Dos"
        say = "¿Cuánto?"
        pregunta = { prompt = "p", opciones = ["a", "b"], correcta = "b", explica = "e" }
    "#;

    #[test]
    fn the_voice_sets_the_clock_and_a_demo_can_stretch_a_beat() {
        let c = parse(SRC).unwrap();
        let l = plan(&c, &[2.0, 1.0, 3.0]);
        assert_eq!(l.tramos[0].t1, 2.5);
        // a = 2 m/s² stops in 4 s: lead + 4 s + tail outlasts the 1 s line
        assert!((l.tramos[1].t1 - (2.5 + DEMO_LEAD + 4.0 + DEMO_TAIL + 0.5)).abs() < 1e-9);
        assert_eq!(l.capitulos.len(), 2);
        assert_eq!(l.capitulos[1].t, l.tramos[2].t0);
        assert_eq!(l.duracion, l.tramos[2].t1);
        assert_eq!(l.tramo_en(l.tramos[1].t0 + 0.1), 1);
    }

    #[test]
    fn cues_cover_the_speech_sentence_by_sentence() {
        let c = parse(SRC).unwrap();
        let l = plan(&c, &[2.0, 1.0, 3.0]);
        let cues = l.cues(&c);
        assert_eq!(cues[0].text, "Hola.");
        assert_eq!(cues[1].text, "¿Listo?");
        assert_eq!(cues[0].t0, 0.0);
        assert_eq!(cues[1].t1, 2.0);
    }

    #[test]
    fn the_player_stops_when_the_question_is_spoken() {
        let c = parse(SRC).unwrap();
        let l = plan(&c, &[2.0, 1.0, 3.0]);
        let p = l.paradas(&c);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].t, round3(l.tramos[2].habla));
        assert_eq!((p[0].chapter, p[0].answer.as_str()), (1, "b"));
    }
}
