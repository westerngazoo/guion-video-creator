//! The studio's HTTP server (R-0015 AC6): std only, HTTP/1.1 by hand, one
//! request at a time, bound to 127.0.0.1.
//!
//! | Request | Answer |
//! |---|---|
//! | `GET /` | the page (embedded) |
//! | `GET /api/estado?t=T` | the state at `T`, JSON ([`crate::Estudio::estado`]) |
//! | `GET /api/cuadro.png?t=T&r=K` | the frame at `T`, reduced `K` times (default 2) |
//! | `GET /api/escena.toml` | `escena.toml` as it would be saved |
//! | `POST /api/param` | `{"id","param","valor","procedencia"?}` → the state, or 400 naming the problem |
//! | `POST /api/guardar` | writes `escena.toml` → `{"guardado": path}` |
//!
//! [`responder`] is the whole router and takes no socket, so the endpoints
//! are tested without a network; [`servir`] only moves bytes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use guion_gadget::{Dato, Procedencia};
use serde_json::{json, Value};

use crate::Estudio;

/// The page, embedded in the binary.
pub const PAGINA: &str = include_str!("ui.html");

/// A request, as far as the studio needs one.
#[derive(Debug, Clone, PartialEq)]
pub struct Peticion {
    pub metodo: String,
    pub ruta: String,
    pub query: Vec<(String, String)>,
    pub cuerpo: Vec<u8>,
}

impl Peticion {
    /// A request without a body (`ruta` may carry `?query`).
    pub fn get(ruta: &str) -> Peticion {
        Peticion::nueva("GET", ruta, Vec::new())
    }

    /// A POST with a body.
    pub fn post(ruta: &str, cuerpo: &str) -> Peticion {
        Peticion::nueva("POST", ruta, cuerpo.as_bytes().to_vec())
    }

    fn nueva(metodo: &str, objetivo: &str, cuerpo: Vec<u8>) -> Peticion {
        let (ruta, q) = objetivo.split_once('?').unwrap_or((objetivo, ""));
        let query = q
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|kv| {
                let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
                (decodificar(k), decodificar(v))
            })
            .collect();
        Peticion {
            metodo: metodo.to_string(),
            ruta: decodificar(ruta),
            query,
            cuerpo,
        }
    }

    fn param(&self, k: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
    }
}

/// `%XX` and `+` decoded (invalid escapes are kept as they are).
fn decodificar(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (
            b[i],
            b.get(i + 1).copied().and_then(hex),
            b.get(i + 2).copied().and_then(hex),
        ) {
            (b'+', _, _) => out.push(b' '),
            (b'%', Some(h), Some(l)) => {
                out.push(h * 16 + l);
                i += 2;
            }
            (c, _, _) => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A response.
#[derive(Debug, Clone, PartialEq)]
pub struct Respuesta {
    pub estado: u16,
    pub tipo: &'static str,
    pub cuerpo: Vec<u8>,
}

impl Respuesta {
    fn json(estado: u16, v: &Value) -> Respuesta {
        Respuesta {
            estado,
            tipo: "application/json; charset=utf-8",
            cuerpo: v.to_string().into_bytes(),
        }
    }

    fn error(estado: u16, m: &str) -> Respuesta {
        Respuesta::json(estado, &json!({ "error": m }))
    }

    /// The body as text (for tests and logs).
    pub fn texto(&self) -> String {
        String::from_utf8_lossy(&self.cuerpo).into_owned()
    }
}

fn tiempo(p: &Peticion) -> Result<f64, String> {
    match p.param("t") {
        None => Ok(0.0),
        Some(s) => match s.parse::<f64>() {
            Ok(t) if t.is_finite() && t >= 0.0 => Ok(t),
            _ => Err(format!("t = «{s}»: seconds ≥ 0")),
        },
    }
}

/// The router: one request in, one response out, the studio's state
/// changed only by `POST /api/param`.
pub fn responder(e: &mut Estudio, p: &Peticion) -> Respuesta {
    match (p.metodo.as_str(), p.ruta.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => Respuesta {
            estado: 200,
            tipo: "text/html; charset=utf-8",
            cuerpo: PAGINA.as_bytes().to_vec(),
        },
        ("GET", "/api/estado") => match tiempo(p).and_then(|t| e.estado(t)) {
            Ok(v) => Respuesta::json(200, &v),
            Err(m) => Respuesta::error(400, &m),
        },
        ("GET", "/api/cuadro.png") => {
            let r = match p.param("r").map(str::parse::<usize>) {
                None => Ok(2),
                Some(Ok(k)) if (1..=8).contains(&k) => Ok(k),
                Some(_) => Err("r: a reduction from 1 to 8".to_string()),
            };
            match r.and_then(|r| tiempo(p).map(|t| (t, r))) {
                Ok((t, r)) => match e.cuadro_png(t, r) {
                    Ok(png) => Respuesta {
                        estado: 200,
                        tipo: "image/png",
                        cuerpo: png,
                    },
                    Err(m) => Respuesta::error(500, &m),
                },
                Err(m) => Respuesta::error(400, &m),
            }
        }
        ("GET", "/api/escena.toml") => Respuesta {
            estado: 200,
            tipo: "text/plain; charset=utf-8",
            cuerpo: e.escena.a_toml(&e.registro).into_bytes(),
        },
        ("POST", "/api/param") => match fijar(e, &p.cuerpo) {
            Ok(()) => match tiempo(p).and_then(|t| e.estado(t)) {
                Ok(v) => Respuesta::json(200, &v),
                Err(m) => Respuesta::error(400, &m),
            },
            Err(m) => Respuesta::error(400, &m),
        },
        ("POST", "/api/guardar") => match e.guardar() {
            Ok(f) => Respuesta::json(200, &json!({ "guardado": f.display().to_string() })),
            Err(m) => Respuesta::error(500, &m),
        },
        _ => Respuesta::error(404, &format!("{} {}: no such endpoint", p.metodo, p.ruta)),
    }
}

/// `{"id","param","valor","procedencia"?}` → [`Estudio::fijar`].
fn fijar(e: &mut Estudio, cuerpo: &[u8]) -> Result<(), String> {
    let v: Value = serde_json::from_slice(cuerpo).map_err(|x| format!("body: {x}"))?;
    let campo = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("body: «{k}» (a text) is missing"))
    };
    let (id, param) = (campo("id")?, campo("param")?);
    let d = match v.get("valor") {
        Some(Value::Number(n)) => Dato::Num(n.as_f64().ok_or("valor: not a finite number")?),
        Some(Value::String(s)) => Dato::Texto(s.clone()),
        _ => return Err("body: «valor» (a number or a text) is missing".into()),
    };
    let pr = match v.get("procedencia").and_then(Value::as_str) {
        Some(s) => Some(Procedencia::de(s)?),
        None => None,
    };
    e.fijar(id, param, d, pr)
}

fn leer(s: &TcpStream) -> std::io::Result<Peticion> {
    let mut r = BufReader::new(s);
    let mut linea = String::new();
    r.read_line(&mut linea)?;
    let mut partes = linea.split_whitespace();
    let (metodo, objetivo) = (
        partes.next().unwrap_or("").to_string(),
        partes.next().unwrap_or("/").to_string(),
    );
    let mut largo = 0usize;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                largo = v.trim().parse().unwrap_or(0);
            }
        }
    }
    // a slider's body is a few bytes; anything past 1 MB is not the page
    let mut cuerpo = vec![0u8; largo.min(1 << 20)];
    r.read_exact(&mut cuerpo)?;
    Ok(Peticion::nueva(&metodo, &objetivo, cuerpo))
}

fn escribir(mut s: &TcpStream, r: &Respuesta) -> std::io::Result<()> {
    let razon = match r.estado {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        s,
        "HTTP/1.1 {} {razon}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        r.estado,
        r.tipo,
        r.cuerpo.len()
    )?;
    s.write_all(&r.cuerpo)?;
    s.flush()
}

fn atender(e: &mut Estudio, s: &TcpStream) {
    let r = match leer(s) {
        Ok(p) => responder(e, &p),
        Err(x) => Respuesta::error(400, &x.to_string()),
    };
    if let Err(x) = escribir(s, &r) {
        eprintln!("guion studio: {x}");
    }
}

/// Serves `e` on `l` until the process ends: one request at a time, each
/// connection closed after its response. A broken connection is logged and
/// skipped; it never stops the studio.
pub fn servir(e: &mut Estudio, l: &TcpListener) {
    for s in l.incoming().flatten() {
        atender(e, &s);
    }
}

/// Serves `n` connections, then returns (the tests use it).
pub fn servir_n(e: &mut Estudio, l: &TcpListener, n: usize) {
    for s in l.incoming().take(n).flatten() {
        atender(e, &s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_la_query() {
        let p = Peticion::get("/api/cuadro.png?t=1.5&r=1&x=a%20b+c&y=%zz");
        assert_eq!(p.ruta, "/api/cuadro.png");
        assert_eq!(p.param("t"), Some("1.5"));
        assert_eq!(p.param("x"), Some("a b c"));
        assert_eq!(p.param("y"), Some("%zz"));
    }
}
