use guion_core::{Narration, Screenplay, Span};

use crate::error::AudioError;

#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub text: String,
    pub at: Span,
}

/// Collect narration cues from `[[narration]]` or an external script file.
pub fn collect_cues(sp: &Screenplay, script_path: Option<&std::path::Path>) -> Result<Vec<Cue>, AudioError> {
    if !sp.narration.is_empty() {
        return Ok(sp
            .narration
            .iter()
            .map(|n| Cue {
                text: n.text.clone(),
                at: n.at,
            })
            .collect());
    }
    if let Some(path) = script_path {
        return parse_script_file(path);
    }
    Err(AudioError::NoCues)
}

/// Parse `templates/foo.narration.txt` blocks: `[0.4–3.8]` then text lines.
pub fn parse_script(src: &str) -> Result<Vec<Cue>, AudioError> {
    let mut cues = Vec::new();
    let mut span: Option<Span> = None;
    let mut lines = Vec::new();

    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(at) = span {
                if !lines.is_empty() {
                    cues.push(Cue {
                        text: lines.join(" "),
                        at,
                    });
                }
            }
            span = Some(parse_span_header(&line[1..line.len() - 1])?);
            lines.clear();
            continue;
        }
        if span.is_none() {
            return Err(AudioError::InvalidScript(
                "línea de texto sin bloque [start–end]".into(),
            ));
        }
        lines.push(line);
    }
    if let Some(at) = span {
        if !lines.is_empty() {
            cues.push(Cue {
                text: lines.join(" "),
                at,
            });
        }
    }
    if cues.is_empty() {
        return Err(AudioError::NoCues);
    }
    Ok(cues)
}

pub fn parse_script_file(path: &std::path::Path) -> Result<Vec<Cue>, AudioError> {
    let src = std::fs::read_to_string(path)?;
    parse_script(&src)
}

fn parse_span_header(inner: &str) -> Result<Span, AudioError> {
    let normalized = inner.replace('–', "-");
    let parts: Vec<&str> = normalized.split('-').collect();
    if parts.len() < 2 {
        return Err(AudioError::InvalidScript(format!("span mal formado: [{inner}]")));
    }
    let start = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| AudioError::InvalidScript(format!("start inválido en [{inner}]")))?;
    let end = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| AudioError::InvalidScript(format!("end inválido en [{inner}]")))?;
    if !(start.is_finite() && end.is_finite() && end > start) {
        return Err(AudioError::InvalidScript(format!("rango inválido [{inner}]")));
    }
    Ok(Span { start, end })
}

impl From<&Narration> for Cue {
    fn from(n: &Narration) -> Self {
        Cue {
            text: n.text.clone(),
            at: n.at,
        }
    }
}
