//! Small text helpers: TOML and JSON strings, written by hand so the files
//! keep their comments and their order (R-0014 AC3: same answers, same bytes),
//! and the scaffold's `{%clave%}` placeholders.

/// A TOML basic string.
pub fn toml_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// A TOML array of strings, on one line.
pub fn toml_lista(v: &[String]) -> String {
    let items: Vec<String> = v.iter().map(|s| toml_str(s)).collect();
    format!("[{}]", items.join(", "))
}

/// A JSON string.
pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Replaces every `{%clave%}` with its value. A placeholder with no value is
/// an error naming it, so a template never ships a hole; `{{…}}` (the
/// placeholders of `guion reel nuevo`, R-0013 AC10) is left alone.
pub fn sustituir(
    plantilla: &str,
    valores: &[(&str, String)],
    nombre: &str,
) -> Result<String, String> {
    let mut out = String::with_capacity(plantilla.len());
    let mut resto = plantilla;
    while let Some(i) = resto.find("{%") {
        out.push_str(&resto[..i]);
        let cola = &resto[i + 2..];
        let j = cola
            .find("%}")
            .ok_or_else(|| format!("{nombre}: «{{%» without «%}}»"))?;
        let clave = &cola[..j];
        let v = valores
            .iter()
            .find(|(k, _)| *k == clave)
            .ok_or_else(|| format!("{nombre}: no value for {{%{clave}%}}"))?;
        out.push_str(&v.1);
        resto = &cola[j + 2..];
    }
    out.push_str(resto);
    Ok(out)
}

/// A name folded to a slug: lowercase ASCII, accents dropped, the rest
/// hyphens («El extraño arte de computar» → `el-extrano-arte-de-computar`).
pub fn a_slug(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            o.push(c);
        } else if !o.ends_with('-') && !o.is_empty() {
            o.push('-');
        }
    }
    while o.ends_with('-') {
        o.pop();
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_escape_and_slugs_fold() {
        assert_eq!(toml_str("a \"b\" \\ c"), r#""a \"b\" \\ c""#);
        assert_eq!(json_str("x\ny"), r#""x\ny""#);
        assert_eq!(
            a_slug("El extraño arte de computar"),
            "el-extrano-arte-de-computar"
        );
        assert_eq!(a_slug("¿Qué?"), "que");
    }

    #[test]
    fn placeholders_fail_by_name_and_leave_the_posts_alone() {
        let v = [("nombre", "X".to_string())];
        assert_eq!(
            sustituir("{%nombre%} {{slug}}", &v, "t").unwrap(),
            "X {{slug}}"
        );
        assert!(sustituir("{%otro%}", &v, "t").unwrap_err().contains("otro"));
    }
}
