//! Binding tokens and the reference table (SPEC-0001 §"bind.rs").
//!
//! A screenplay connects geometry, labels and motion to model/object outputs
//! with `@` tokens, the same idea as `physics-lab`'s readout slots. The grammar
//! is deliberately tiny (R-0001 Q2): `@id` or `@id.field`, where `id` and
//! `field` are plain identifiers. Anchor sub-selectors (`at = "mid"`) are typed
//! fields elsewhere, not part of the token grammar.
//!
//! Two responsibilities live here so nothing else re-implements them:
//!
//! * [`Token`] owns the *grammar* — it validates at deserialization time, so a
//!   malformed token fails loudly at load (surfaced as `LoadError::Parse`).
//! * [`SymbolTable`] owns *resolution* — the set of declared ids, so
//!   `validate.rs` can decide whether a token dangles (R-0001 AC6).

use std::collections::BTreeSet;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::Screenplay;

/// A validated binding token, stored verbatim including the leading `@`.
///
/// Constructed only through [`Token::parse`] (and thus `Deserialize`), so any
/// `Token` in the model is guaranteed to be grammatically `@id` or `@id.field`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token(String);

impl Token {
    /// Parse and validate the grammar. The referenced id is *not* checked here
    /// (that is resolution, in `validate.rs`); only the shape of the string.
    pub fn parse(raw: &str) -> Result<Token, String> {
        let rest = raw
            .strip_prefix('@')
            .ok_or_else(|| format!("un binding debe empezar con '@': {raw:?}"))?;
        if rest.is_empty() {
            return Err(format!("binding vacío: {raw:?}"));
        }
        let mut parts = rest.splitn(2, '.');
        let id = parts.next().unwrap_or("");
        valid_ident(id).map_err(|e| format!("id inválido en {raw:?}: {e}"))?;
        if let Some(field) = parts.next() {
            if field.contains('.') {
                return Err(format!("demasiados '.' en {raw:?}: sólo @id o @id.field"));
            }
            valid_ident(field).map_err(|e| format!("field inválido en {raw:?}: {e}"))?;
        }
        Ok(Token(raw.to_string()))
    }

    /// The id part: `@lever.phi` → `lever`, `@forearm` → `forearm`.
    pub fn id(&self) -> &str {
        let rest = &self.0[1..];
        match rest.find('.') {
            Some(i) => &rest[..i],
            None => rest,
        }
    }

    /// The optional field part: `@lever.phi` → `Some("phi")`.
    pub fn field(&self) -> Option<&str> {
        let rest = &self.0[1..];
        rest.find('.').map(|i| &rest[i + 1..])
    }

    /// The full token text, `@` included.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Every well-formed `@` token embedded in a free string, in order.
    ///
    /// Lets `validate.rs` resolve tokens that live inside otherwise opaque
    /// strings, e.g. a `heat:@lever.force_N` stroke paint. Malformed `@…`
    /// fragments are skipped rather than raised — a free string is not a token
    /// field, so only its resolvable tokens matter.
    pub fn extract(s: &str) -> Vec<Token> {
        let bytes = s.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'@' {
                let start = i;
                i += 1;
                while i < bytes.len() && is_ident_or_dot(bytes[i]) {
                    i += 1;
                }
                if let Ok(t) = Token::parse(&s[start..i]) {
                    out.push(t);
                }
            } else {
                i += 1;
            }
        }
        out
    }
}

fn is_ident_or_dot(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

fn valid_ident(s: &str) -> Result<(), String> {
    let mut chars = s.chars();
    match chars.next() {
        None => return Err("identificador vacío".to_string()),
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        Some(c) => return Err(format!("debe empezar con letra o '_', no {c:?}")),
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("carácter inválido {c:?}"));
        }
    }
    Ok(())
}

impl Serialize for Token {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Token::parse(&s).map_err(D::Error::custom)
    }
}

/// The declared ids a token may resolve against: every `model.id` and
/// `object.id` in the screenplay.
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    ids: BTreeSet<String>,
}

impl SymbolTable {
    /// Collect the declared model and object ids.
    pub fn build(sp: &Screenplay) -> Self {
        let mut ids = BTreeSet::new();
        for m in &sp.model {
            ids.insert(m.id.clone());
        }
        for o in &sp.object {
            ids.insert(o.id.clone());
        }
        SymbolTable { ids }
    }

    /// Whether `id` is declared.
    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_id_and_field() {
        let t = Token::parse("@lever.phi").unwrap();
        assert_eq!(t.id(), "lever");
        assert_eq!(t.field(), Some("phi"));
        assert_eq!(t.as_str(), "@lever.phi");

        let t = Token::parse("@forearm").unwrap();
        assert_eq!(t.id(), "forearm");
        assert_eq!(t.field(), None);
    }

    #[test]
    fn rejects_malformed_tokens() {
        for bad in ["elbow", "@", "@1bad", "@a.b.c", "@a-b", "@.x"] {
            assert!(Token::parse(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn extracts_embedded_tokens() {
        let got = Token::extract("heat:@lever.force_N");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].as_str(), "@lever.force_N");

        assert!(Token::extract("skin").is_empty());
    }
}
