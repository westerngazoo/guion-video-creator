//! The dashboard's screen and keys (R-0013 AC4), with no terminal in it:
//! [`App`] holds the state and turns keys into [`Efecto`]s; [`dibujar`] draws
//! it on any ratatui backend (the tests use `TestBackend`).
//!
//! ```text
//! ┌ cuentas ┐┌ posts ─────────────────────┐┌ detalle ──────────┐
//! │ Todas   ││   estado    post  check dur ││ title, estado,    │
//! │ A  3    ││ ● listo     x     PASS 87 s ││ check, paths,     │
//! └─────────┘└────────────────────────────┘└ encargos ─────────┘
//! ┌ registro ─────────────────────────────────────────────────┐
//! └───────────────────────────────────────────────────────────┘
//!  keys / prompt
//! ```

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap,
};
use ratatui::Frame;

use crate::acciones::{self, Abrir, Accion, Comando};
use crate::nuevo;
use crate::post::{Estado, Post};
use crate::tablero::{Cuenta, Tablero};
use crate::trabajos::{Evento, Trabajo};

/// Which pane the arrows move in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foco {
    Cuentas,
    Posts,
}

/// A one-line form at the bottom.
#[derive(Debug, Clone, PartialEq)]
pub struct Prompt {
    pub tipo: TipoPrompt,
    /// (label, value) per field.
    pub campos: Vec<(&'static str, String)>,
    pub actual: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TipoPrompt {
    /// `p`: date and link for these posts.
    Publicado(Vec<PathBuf>),
    /// `e`: account, idea and (optionally) a slug for `guion reel nuevo`.
    Encargo,
}

/// What the terminal loop must do after a key.
#[derive(Debug, Clone, PartialEq)]
pub enum Efecto {
    Lanzar(Vec<Trabajo>),
    Abrir(Comando),
    Releer,
}

/// The dashboard's state.
pub struct App {
    pub tablero: Tablero,
    pub foco: Foco,
    /// 0: every account; i + 1: account i.
    pub cuenta_sel: usize,
    /// Index into [`App::visibles`].
    pub post_sel: usize,
    pub filtro: Option<Estado>,
    /// Marked posts, by folder.
    pub marcados: BTreeSet<PathBuf>,
    pub log: VecDeque<String>,
    /// Posts with a job running, and which.
    pub corriendo: HashMap<PathBuf, Accion>,
    pub prompt: Option<Prompt>,
    pub salir: bool,
    pub guion: PathBuf,
    pub os: &'static str,
    pub pantalla: bool,
    pub hoy: String,
}

const LOG_MAX: usize = 1000;

impl App {
    pub fn new(tablero: Tablero, guion: PathBuf, hoy: String) -> App {
        App {
            tablero,
            foco: Foco::Posts,
            cuenta_sel: 0,
            post_sel: 0,
            filtro: None,
            marcados: BTreeSet::new(),
            log: VecDeque::new(),
            corriendo: HashMap::new(),
            prompt: None,
            salir: false,
            guion,
            os: std::env::consts::OS,
            pantalla: acciones::hay_pantalla(),
            hoy,
        }
    }

    pub fn registrar(&mut self, s: impl Into<String>) {
        self.log.push_back(s.into());
        while self.log.len() > LOG_MAX {
            self.log.pop_front();
        }
    }

    fn cuenta_filtro(&self) -> Option<&str> {
        self.cuenta_sel
            .checked_sub(1)
            .and_then(|i| self.tablero.cuentas.get(i))
            .map(|c| c.nombre.as_str())
    }

    /// The posts the middle pane lists: (account, post).
    pub fn visibles(&self) -> Vec<(usize, usize)> {
        self.tablero.filtrar(self.cuenta_filtro(), self.filtro)
    }

    fn post(&self, (i, j): (usize, usize)) -> (&Cuenta, &Post) {
        let c = &self.tablero.cuentas[i];
        (c, &c.posts[j])
    }

    /// The post under the cursor.
    pub fn actual(&self) -> Option<(usize, usize)> {
        self.visibles().get(self.post_sel).copied()
    }

    /// What an action applies to: the marked posts, else the cursor's.
    pub fn objetivos(&self) -> Vec<(usize, usize)> {
        if self.marcados.is_empty() {
            return self.actual().into_iter().collect();
        }
        self.tablero
            .filtrar(None, None)
            .into_iter()
            .filter(|&ij| self.marcados.contains(&self.post(ij).1.dir))
            .collect()
    }

    /// Swaps in a re-read state, keeping the cursor on the same post.
    pub fn reemplazar(&mut self, t: Tablero) {
        let antes = self.actual().map(|ij| self.post(ij).1.dir.clone());
        self.tablero = t;
        self.cuenta_sel = self.cuenta_sel.min(self.tablero.cuentas.len());
        let vis = self.visibles();
        self.post_sel = antes
            .and_then(|d| vis.iter().position(|&ij| self.post(ij).1.dir == d))
            .unwrap_or(self.post_sel.min(vis.len().saturating_sub(1)));
    }

    /// A job's report. `true` when the state should be re-read (a job ended).
    pub fn evento(&mut self, e: Evento) -> bool {
        match e {
            Evento::Empieza {
                dir,
                post,
                accion,
                comando,
            } => {
                self.corriendo.insert(dir, accion);
                self.registrar(format!("[{post}] $ {comando}"));
                false
            }
            Evento::Linea { post, texto } => {
                self.registrar(format!("[{post}] {texto}"));
                false
            }
            Evento::Fin {
                dir,
                post,
                accion,
                ok,
                codigo,
            } => {
                self.corriendo.remove(&dir);
                let fin = match (ok, codigo) {
                    (true, _) => "ok".to_string(),
                    (false, Some(c)) => format!("falló (salida {c})"),
                    (false, None) => "falló".to_string(),
                };
                self.registrar(format!("[{post}] {} {fin}", accion.nombre()));
                true
            }
        }
    }

    fn mover(&mut self, d: isize) {
        match self.foco {
            Foco::Cuentas => {
                let n = self.tablero.cuentas.len() + 1;
                self.cuenta_sel = (self.cuenta_sel as isize + d).rem_euclid(n as isize) as usize;
                self.post_sel = 0;
            }
            Foco::Posts => {
                let n = self.visibles().len();
                if n > 0 {
                    self.post_sel = (self.post_sel as isize + d).clamp(0, n as isize - 1) as usize;
                }
            }
        }
    }

    fn lanzar(&mut self, accion: Accion) -> Vec<Efecto> {
        let mut trabajos = Vec::new();
        for ij in self.objetivos() {
            let (c, p) = self.post(ij);
            if self.corriendo.contains_key(&p.dir) {
                let m = format!("[{}] ya corre algo; se omite", p.nombre);
                self.registrar(m);
                continue;
            }
            match acciones::comando(accion, p, c, &self.guion) {
                Ok(comando) => trabajos.push(Trabajo {
                    accion,
                    post: p.nombre.clone(),
                    dir: p.dir.clone(),
                    comando,
                }),
                Err(e) => self.registrar(format!("{} omitido: {e}", accion.nombre())),
            }
        }
        if trabajos.is_empty() {
            Vec::new()
        } else {
            vec![Efecto::Lanzar(trabajos)]
        }
    }

    fn abrir(&mut self) -> Vec<Efecto> {
        let Some(ij) = self.actual() else {
            return Vec::new();
        };
        let (c, p) = self.post(ij);
        match acciones::abrir(p, c, self.os, self.pantalla) {
            Ok(Abrir::Con(cmd)) => {
                self.registrar(format!("$ {}", cmd.mostrar()));
                vec![Efecto::Abrir(cmd)]
            }
            Ok(Abrir::Ruta(f)) => {
                self.registrar(format!("sin pantalla: {}", f.display()));
                Vec::new()
            }
            Err(e) => {
                self.registrar(e);
                Vec::new()
            }
        }
    }

    /// A key; returns what the loop must do.
    pub fn tecla(&mut self, k: KeyEvent) -> Vec<Efecto> {
        if self.prompt.is_some() {
            return self.tecla_prompt(k);
        }
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
            self.salir = true;
            return Vec::new();
        }
        match k.code {
            KeyCode::Char('q') | KeyCode::Esc => self.salir = true,
            KeyCode::Up | KeyCode::Char('k') => self.mover(-1),
            KeyCode::Down | KeyCode::Char('j') => self.mover(1),
            KeyCode::PageUp => self.mover(-10),
            KeyCode::PageDown => self.mover(10),
            KeyCode::Left | KeyCode::Char('h') => self.foco = Foco::Cuentas,
            KeyCode::Right | KeyCode::Char('l') => self.foco = Foco::Posts,
            KeyCode::Tab => {
                self.foco = match self.foco {
                    Foco::Cuentas => Foco::Posts,
                    Foco::Posts => Foco::Cuentas,
                }
            }
            KeyCode::Char(' ') => {
                if let Some(ij) = self.actual() {
                    let d = self.post(ij).1.dir.clone();
                    if !self.marcados.remove(&d) {
                        self.marcados.insert(d);
                    }
                    self.mover(1);
                }
            }
            KeyCode::Char('a') => {
                let dirs: Vec<PathBuf> = self
                    .visibles()
                    .into_iter()
                    .map(|ij| self.post(ij).1.dir.clone())
                    .collect();
                if dirs.iter().all(|d| self.marcados.contains(d)) {
                    dirs.iter().for_each(|d| {
                        self.marcados.remove(d);
                    });
                } else {
                    self.marcados.extend(dirs);
                }
            }
            KeyCode::Char('f') => {
                self.filtro = match self.filtro {
                    None => Some(Estado::TODOS[0]),
                    Some(e) => Estado::TODOS
                        .iter()
                        .position(|&x| x == e)
                        .and_then(|i| Estado::TODOS.get(i + 1))
                        .copied(),
                };
                self.post_sel = 0;
            }
            KeyCode::Char('r') => return self.lanzar(Accion::Render),
            KeyCode::Char('c') => return self.lanzar(Accion::Check),
            KeyCode::Char('v') => return self.lanzar(Accion::Voz),
            KeyCode::Char('o') => return self.abrir(),
            KeyCode::Char('g') => return vec![Efecto::Releer],
            KeyCode::Char('p') => {
                let dirs: Vec<PathBuf> = self
                    .objetivos()
                    .into_iter()
                    .map(|ij| self.post(ij).1.dir.clone())
                    .collect();
                if !dirs.is_empty() {
                    self.prompt = Some(Prompt {
                        tipo: TipoPrompt::Publicado(dirs),
                        campos: vec![("fecha", self.hoy.clone()), ("enlace", String::new())],
                        actual: 0,
                    });
                }
            }
            KeyCode::Char('e') => {
                let cuenta = self
                    .cuenta_filtro()
                    .map(str::to_string)
                    .or_else(|| self.actual().map(|ij| self.post(ij).0.nombre.clone()))
                    .unwrap_or_default();
                self.prompt = Some(Prompt {
                    tipo: TipoPrompt::Encargo,
                    campos: vec![
                        ("cuenta", cuenta),
                        ("idea", String::new()),
                        ("slug (vacío: solo el encargo)", String::new()),
                    ],
                    actual: 1,
                });
            }
            _ => {}
        }
        Vec::new()
    }

    fn tecla_prompt(&mut self, k: KeyEvent) -> Vec<Efecto> {
        let nombres: Vec<String> = self
            .tablero
            .cuentas
            .iter()
            .map(|c| c.nombre.clone())
            .collect();
        let Some(pr) = self.prompt.as_mut() else {
            return Vec::new();
        };
        match k.code {
            KeyCode::Esc => {
                self.prompt = None;
                self.registrar("cancelado");
            }
            KeyCode::Backspace => {
                pr.campos[pr.actual].1.pop();
            }
            KeyCode::Tab
                if pr.tipo == TipoPrompt::Encargo && pr.actual == 0 && !nombres.is_empty() =>
            {
                let i = nombres
                    .iter()
                    .position(|n| *n == pr.campos[0].1)
                    .map_or(0, |i| (i + 1) % nombres.len());
                pr.campos[0].1 = nombres[i].clone();
            }
            KeyCode::Up | KeyCode::BackTab => pr.actual = pr.actual.saturating_sub(1),
            KeyCode::Enter | KeyCode::Down if pr.actual + 1 < pr.campos.len() => pr.actual += 1,
            KeyCode::Enter => {
                let pr = self.prompt.take().expect("a prompt");
                return self.enviar(pr);
            }
            KeyCode::Char(ch) => pr.campos[pr.actual].1.push(ch),
            _ => {}
        }
        Vec::new()
    }

    fn enviar(&mut self, pr: Prompt) -> Vec<Efecto> {
        match pr.tipo {
            TipoPrompt::Publicado(dirs) => {
                let (fecha, enlace) = (pr.campos[0].1.trim(), pr.campos[1].1.trim());
                for ij in self.tablero.filtrar(None, None) {
                    let p = self.post(ij).1;
                    if !dirs.contains(&p.dir) {
                        continue;
                    }
                    let m = match acciones::publicar(p, fecha, enlace) {
                        Ok(()) => format!("[{}] publicado ({fecha})", p.nombre),
                        Err(e) => e,
                    };
                    self.registrar(m);
                }
                self.marcados.clear();
            }
            TipoPrompt::Encargo => {
                let (cuenta, idea, slug) = (
                    pr.campos[0].1.trim(),
                    pr.campos[1].1.trim(),
                    pr.campos[2].1.trim(),
                );
                let mut post = None;
                if !slug.is_empty() {
                    match self
                        .tablero
                        .cuenta(cuenta)
                        .ok_or(format!("no hay cuenta «{cuenta}»"))
                        .and_then(|c| nuevo::crear(c, slug, Some(idea), &self.hoy))
                    {
                        Ok(n) => {
                            self.registrar(format!(
                                "nuevo post: {} ({} archivos)",
                                n.dir.display(),
                                n.archivos.len()
                            ));
                            post = Some(slug);
                        }
                        Err(e) => self.registrar(format!("nuevo: {e}")),
                    }
                }
                let m = match self.tablero.encargar(cuenta, idea, &self.hoy, post) {
                    Ok(e) => format!("encargo {} para {}: {}", e.id, e.cuenta, e.idea),
                    Err(e) => format!("encargo: {e}"),
                };
                self.registrar(m);
            }
        }
        vec![Efecto::Releer]
    }
}

/// The colour an estado reads in (terminal colours, not a brand's).
pub fn color(e: Option<Estado>) -> Color {
    match e {
        Some(Estado::Idea) => Color::Gray,
        Some(Estado::Borrador) => Color::Yellow,
        Some(Estado::Revision) => Color::Magenta,
        Some(Estado::Listo) => Color::Green,
        Some(Estado::Publicado) => Color::Cyan,
        None => Color::DarkGray,
    }
}

fn estado_txt(e: Option<Estado>) -> &'static str {
    e.map_or("—", Estado::nombre)
}

fn check_txt(p: &Post) -> (String, Color) {
    match &p.check {
        Some(c) if c.pass => ("PASS".into(), Color::Green),
        Some(c) => (format!("FAIL {}", c.fallas), Color::Red),
        None => ("—".into(), Color::DarkGray),
    }
}

fn dur_txt(p: &Post) -> String {
    p.duracion.map_or("—".into(), |d| format!("{d:.1} s"))
}

fn borde(titulo: String, activo: bool) -> Block<'static> {
    let b = Block::default().borders(Borders::ALL).title(titulo);
    if activo {
        b.border_style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        b.border_style(Style::default().fg(Color::DarkGray))
    }
}

fn corto(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        s.chars().take(n.saturating_sub(1)).collect::<String>() + "…"
    }
}

/// Draws the whole screen.
pub fn dibujar(f: &mut Frame, app: &App) {
    let [principal, registro, pie] = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(8),
        Constraint::Length(1),
    ])
    .areas(f.area());
    let [izq, medio, der] = Layout::horizontal([
        Constraint::Percentage(22),
        Constraint::Percentage(43),
        Constraint::Percentage(35),
    ])
    .areas(principal);
    dibujar_cuentas(f, app, izq);
    dibujar_posts(f, app, medio);
    dibujar_detalle(f, app, der);
    dibujar_registro(f, app, registro);
    dibujar_pie(f, app, pie);
}

fn dibujar_cuentas(f: &mut Frame, app: &App, area: Rect) {
    let total: usize = app.tablero.cuentas.iter().map(|c| c.posts.len()).sum();
    let mut items = vec![ListItem::new(Line::from(format!("Todas  {total}")))];
    for c in &app.tablero.cuentas {
        let pendientes = app
            .tablero
            .pedidos_de(c)
            .filter(|e| e.vivo == "pendiente")
            .count();
        let mut conteo: Vec<Span> = vec![Span::raw("  ")];
        for e in Estado::TODOS {
            let n = c.conteo.de(Some(e));
            conteo.push(Span::styled(
                format!("{}{n} ", &e.nombre()[..1]),
                Style::default().fg(color(Some(e))),
            ));
        }
        if pendientes > 0 {
            conteo.push(Span::raw(format!("E{pendientes}")));
        }
        items.push(ListItem::new(vec![
            Line::from(format!("{}  {}", c.nombre, c.posts.len())),
            Line::from(conteo),
        ]));
    }
    let lista = List::new(items)
        .block(borde(" cuentas ".into(), app.foco == Foco::Cuentas))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut st = ListState::default();
    st.select(Some(app.cuenta_sel));
    f.render_stateful_widget(lista, area, &mut st);
}

fn dibujar_posts(f: &mut Frame, app: &App, area: Rect) {
    let todas = app.cuenta_sel == 0;
    let vis = app.visibles();
    let filas: Vec<Row> = vis
        .iter()
        .map(|&ij| {
            let (c, p) = app.post(ij);
            let marca = if app.corriendo.contains_key(&p.dir) {
                "…"
            } else if app.marcados.contains(&p.dir) {
                "●"
            } else {
                " "
            };
            let (chk, chk_c) = check_txt(p);
            let mut celdas = vec![
                Cell::from(marca),
                Cell::from(estado_txt(p.estado)).style(Style::default().fg(color(p.estado))),
            ];
            if todas {
                celdas.push(Cell::from(corto(&c.nombre, 10)));
            }
            celdas.extend([
                Cell::from(p.nombre.clone()),
                Cell::from(chk).style(Style::default().fg(chk_c)),
                Cell::from(dur_txt(p)),
                Cell::from(if p.narrado { "sí" } else { "—" }),
                Cell::from(p.tipo()),
            ]);
            Row::new(celdas)
        })
        .collect();
    let mut anchos = vec![Constraint::Length(1), Constraint::Length(9)];
    let mut cab = vec!["", "estado"];
    if todas {
        anchos.push(Constraint::Length(10));
        cab.push("cuenta");
    }
    anchos.extend([
        Constraint::Min(10),
        Constraint::Length(6),
        Constraint::Length(7),
        Constraint::Length(3),
        Constraint::Length(6),
    ]);
    cab.extend(["post", "check", "dur", "voz", "tipo"]);
    let filtro = app
        .filtro
        .map_or("todos".to_string(), |e| e.nombre().to_string());
    let titulo = format!(
        " posts · {} · filtro: {filtro} · marcados: {} ",
        vis.len(),
        app.marcados.len()
    );
    let tabla = Table::new(filas, anchos)
        .header(Row::new(cab).style(Style::default().add_modifier(Modifier::BOLD)))
        .block(borde(titulo, app.foco == Foco::Posts))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut st = TableState::default();
    st.select((!vis.is_empty()).then_some(app.post_sel));
    f.render_stateful_widget(tabla, area, &mut st);
}

fn dibujar_detalle(f: &mut Frame, app: &App, area: Rect) {
    let mut l: Vec<Line> = Vec::new();
    if let Some(ij) = app.actual() {
        let (c, p) = app.post(ij);
        let gris = Style::default().fg(Color::DarkGray);
        l.push(Line::styled(
            p.titulo.clone().unwrap_or_else(|| p.nombre.clone()),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        l.push(Line::from(vec![
            Span::styled("cuenta ", gris),
            Span::raw(c.nombre.clone()),
        ]));
        l.push(Line::from(vec![
            Span::styled("estado ", gris),
            Span::styled(
                estado_txt(p.estado),
                Style::default()
                    .fg(color(p.estado))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        if let Some(n) = &p.nota {
            l.push(Line::from(corto(n, 280)));
        }
        l.push(Line::from(vec![
            Span::styled("tipo ", gris),
            Span::raw(format!(
                "{} · voz {} · {}",
                p.tipo(),
                if p.narrado { "sí" } else { "no" },
                dur_txt(p)
            )),
        ]));
        l.push(Line::raw(""));
        match &p.check {
            Some(k) => {
                let (t, col) = check_txt(p);
                l.push(Line::from(vec![
                    Span::styled("check ", gris),
                    Span::styled(
                        format!("{t} ({} de {} pasan)", k.total - k.fallas, k.total),
                        Style::default().fg(col),
                    ),
                ]));
                let (malas, buenas): (Vec<&String>, Vec<&String>) =
                    k.lineas.iter().partition(|x| x.starts_with("FAIL"));
                for x in malas.into_iter().chain(buenas) {
                    let st = if x.starts_with("FAIL") {
                        Style::default().fg(Color::Red)
                    } else {
                        gris
                    };
                    l.push(Line::styled(x.clone(), st));
                }
            }
            None => l.push(Line::from(vec![
                Span::styled("check ", gris),
                Span::raw("— (sin check.json)"),
            ])),
        }
        l.push(Line::raw(""));
        for (k, v) in [
            ("carpeta", Some(&p.ruta)),
            ("video", p.video.as_ref()),
            ("hoja", p.hoja.as_ref()),
            ("publicar", p.publicar.as_ref()),
        ] {
            l.push(Line::from(vec![
                Span::styled(format!("{k} "), gris),
                Span::raw(v.cloned().unwrap_or_else(|| "—".into())),
            ]));
        }
        // the open ones: still pending, or taken and on their way (AC12)
        let encargos: Vec<_> = app
            .tablero
            .pedidos_de(c)
            .filter(|e| e.vivo == "pendiente" || e.vivo == "en curso")
            .collect();
        if !encargos.is_empty() {
            l.push(Line::raw(""));
            l.push(Line::styled(
                format!("encargos abiertos de {}", c.nombre),
                gris,
            ));
            for e in encargos {
                let en_curso = match (
                    &e.vivo[..],
                    e.post_ruta.as_deref().or(e.encargo.post.as_deref()),
                ) {
                    ("en curso", Some(p)) => format!("  → {p}"),
                    ("en curso", None) => "  (en curso)".into(),
                    _ => String::new(),
                };
                let e2 = &e.encargo;
                l.push(Line::from(format!(
                    "{} {} {}{en_curso}",
                    e2.id, e2.fecha, e2.idea
                )));
            }
        }
    } else {
        l.push(Line::raw("sin posts con este filtro"));
    }
    let p = Paragraph::new(l)
        .block(borde(" detalle ".into(), false))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn dibujar_registro(f: &mut Frame, app: &App, area: Rect) {
    let alto = area.height.saturating_sub(2) as usize;
    let lineas: Vec<Line> = app
        .log
        .iter()
        .skip(app.log.len().saturating_sub(alto))
        .map(|s| Line::raw(s.clone()))
        .collect();
    let corriendo = if app.corriendo.is_empty() {
        String::new()
    } else {
        format!(" · corriendo: {}", app.corriendo.len())
    };
    f.render_widget(
        Paragraph::new(lineas).block(borde(format!(" registro{corriendo} "), false)),
        area,
    );
}

fn dibujar_pie(f: &mut Frame, app: &App, area: Rect) {
    let linea = match &app.prompt {
        Some(pr) => {
            let mut spans = Vec::new();
            for (i, (k, v)) in pr.campos.iter().enumerate() {
                let st = if i == pr.actual { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
                spans.push(Span::raw(format!(" {k}: ")));
                spans.push(Span::styled(format!("{v}{}", if i == pr.actual { "▏" } else { "" }), st));
            }
            spans.push(Span::styled("  Enter sigue · Esc cancela", Style::default().fg(Color::DarkGray)));
            Line::from(spans)
        }
        None => Line::styled(
            " ↑↓ mover  ←→ panel  espacio marcar  a todos  f filtro  r render  c check  v voz  o abrir  p publicado  e encargo  g releer  q salir",
            Style::default().fg(Color::DarkGray),
        ),
    };
    f.render_widget(Paragraph::new(linea), area);
}
