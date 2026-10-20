//! The registry (R-0015 AC3): the built-in gadgets plus the ones an account
//! adds, and the contract test every gadget must pass.
//!
//! An account's gadgets are Rust in the account's own motor crate (they
//! draw its kart, its medallion). Its `cuenta.toml` names them in
//! `[estudio] gadgets`, and the motor's `estudio` binary builds a registry
//! with [`Registro::integrados`] plus its own and hands it to the studio
//! (`guion_estudio::run_con`). `guion studio` with a post that uses one says
//! which binary to run.

use guion_reel::Canvas;

use crate::gadget::{Categoria, Contexto, Gadget, Paleta, Salida};
use crate::integrados::{Medidor, Palanca};
use crate::valor::{Dato, Params, Tipo};

/// The gadgets a studio knows, by `tipo`, in the order registered.
pub struct Registro {
    gadgets: Vec<Box<dyn Gadget>>,
}

/// A gadget type name: lowercase letters, digits and hyphens, starting with a letter.
pub fn tipo_valido(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_lowercase())
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl Registro {
    /// An empty registry.
    pub fn vacio() -> Registro {
        Registro {
            gadgets: Vec::new(),
        }
    }

    /// The built-in gadgets.
    pub fn integrados() -> Registro {
        let mut r = Registro::vacio();
        for g in [Box::new(Palanca) as Box<dyn Gadget>, Box::new(Medidor)] {
            r.registrar(g)
                .expect("the built-ins have distinct, valid names");
        }
        r
    }

    /// Adds a gadget; a name that is not a slug, or one already taken, fails
    /// naming it.
    pub fn registrar(&mut self, g: Box<dyn Gadget>) -> Result<(), String> {
        let t = g.tipo();
        if !tipo_valido(t) {
            return Err(format!(
                "gadget «{t}»: a tipo is lowercase letters, digits and hyphens, starting with a letter"
            ));
        }
        if self.buscar(t).is_some() {
            return Err(format!("gadget «{t}» is already registered"));
        }
        self.gadgets.push(g);
        Ok(())
    }

    /// The gadget named `tipo`.
    pub fn buscar(&self, tipo: &str) -> Option<&dyn Gadget> {
        self.gadgets
            .iter()
            .find(|g| g.tipo() == tipo)
            .map(|g| g.as_ref())
    }

    /// Every gadget, shelf by shelf (in the shelves' order), then in the
    /// order registered.
    pub fn todos(&self) -> Vec<&dyn Gadget> {
        let mut v: Vec<&dyn Gadget> = self.gadgets.iter().map(|g| g.as_ref()).collect();
        v.sort_by_key(|g| g.categoria());
        v
    }

    /// The names, in [`Registro::todos`] order (for an error message).
    pub fn nombres(&self) -> Vec<&'static str> {
        self.todos().iter().map(|g| g.tipo()).collect()
    }
}

/// The context the contract test evaluates in: a vertical reel, the
/// neutral palette, no faces, the safe area 60 px in from each edge.
pub fn contexto_de_prueba(paleta: &Paleta) -> Contexto<'_> {
    let c = Canvas::VERTICAL_REEL;
    Contexto {
        canvas: c,
        paleta,
        caras: None,
        cara: 0,
        seguro: [60.0, 60.0, c.width as f64 - 60.0, c.height as f64 - 60.0],
    }
}

/// Evaluates `g` at `t` with its defaults and `cambios` (numbers only).
pub fn eval_con(g: &dyn Gadget, t: f64, cambios: &[(&str, f64)], cx: &Contexto) -> Salida {
    let mut p = Params::defectos(g.params());
    for (n, x) in cambios {
        let pr = p.procedencia(n);
        p.fijar(n, Dato::Num(*x), pr);
    }
    g.eval(t, &p, cx)
}

/// The contract test (R-0015 AC1) for one gadget; each failure is a
/// sentence naming the gadget. An account runs it on its own gadgets.
///
/// - the params: distinct names, every default admitted by its own type,
///   physical params are numbers, ranges ordered;
/// - every oracle reproduces its number within its tolerance;
/// - `eval` is pure: twice with the same `t` and params, the same output;
/// - every value it returns is a declared output, and every output is returned.
pub fn comprobar_contrato(g: &dyn Gadget) -> Vec<String> {
    let mut f = Vec::new();
    let t = g.tipo();
    let mut vistos = Vec::new();
    for s in g.params() {
        if vistos.contains(&s.nombre) {
            f.push(format!("{t}: the param «{}» is declared twice", s.nombre));
        }
        vistos.push(s.nombre);
        if let Err(e) = s.admite(&s.defecto()) {
            f.push(format!("{t}.{}: its default does not fit: {e}", s.nombre));
        }
        if let Tipo::Num { min, max, paso, .. } = s.tipo {
            if !(min <= max && paso > 0.0) {
                f.push(format!("{t}.{}: needs min ≤ max and paso > 0", s.nombre));
            }
        } else if s.fisico {
            f.push(format!(
                "{t}.{}: a physical param is a number with a unit",
                s.nombre
            ));
        }
    }
    let paleta = Paleta::neutra();
    let cx = contexto_de_prueba(&paleta);
    for o in g.oraculos() {
        let s = eval_con(g, 0.0, &o.params, &cx);
        match s.valores.iter().find(|v| v.nombre == o.salida) {
            Some(v) if (v.cantidad - o.esperado).abs() <= o.tolerancia => {}
            Some(v) => f.push(format!(
                "{t}: oracle «{}»: {} = {} instead of {} ± {} ({})",
                o.nombre, o.salida, v.cantidad, o.esperado, o.tolerancia, o.fuente
            )),
            None => f.push(format!(
                "{t}: oracle «{}» checks «{}», which eval does not return",
                o.nombre, o.salida
            )),
        }
    }
    let a = eval_con(g, 0.5, &[], &cx);
    let b = eval_con(g, 0.5, &[], &cx);
    if format!("{a:?}") != format!("{b:?}") {
        f.push(format!(
            "{t}: eval is not pure (same t and params, another output)"
        ));
    }
    let declaradas: Vec<&str> = g.salidas().iter().map(|s| s.nombre).collect();
    for v in &a.valores {
        if !declaradas.contains(&v.nombre.as_str()) {
            f.push(format!(
                "{t}: eval returns «{}», not a declared output",
                v.nombre
            ));
        }
    }
    for d in &declaradas {
        if !a.valores.iter().any(|v| v.nombre == *d) {
            f.push(format!(
                "{t}: the output «{d}» is declared but not returned"
            ));
        }
    }
    f
}

/// The shelves, in order, each with its gadgets (the library panel).
pub fn estantes(r: &Registro) -> Vec<(Categoria, Vec<&dyn Gadget>)> {
    let mut out: Vec<(Categoria, Vec<&dyn Gadget>)> = Vec::new();
    for g in r.todos() {
        match out.last_mut() {
            Some((c, v)) if *c == g.categoria() => v.push(g),
            _ => out.push((g.categoria(), vec![g])),
        }
    }
    out
}
