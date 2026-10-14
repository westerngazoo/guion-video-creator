//! The two JSON pieces `events.json` needs, written by hand so the bytes are
//! the schema's (six decimals, no whitespace) and not a serializer's choice.

use std::fmt::Write;

/// A finite number with exactly six decimals. The callers reject non-finite
/// numbers first ([`crate::Clock`]), since JSON has no NaN.
pub(crate) fn number(out: &mut String, x: f64) {
    debug_assert!(x.is_finite(), "events.json number {x}");
    let _ = write!(out, "{x:.6}");
}

/// A string literal (RFC 8259 §7): `"` and `\` escaped, control characters as `\u00XX`.
pub(crate) fn string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_six_decimals() {
        let mut s = String::new();
        number(&mut s, 5.316671234);
        s.push(' ');
        number(&mut s, 30.0);
        assert_eq!(s, "5.316671 30.000000");
    }

    #[test]
    fn strings_escape_what_rfc_8259_requires() {
        let mut s = String::new();
        string(&mut s, "voz:\"q\\e\"\n");
        assert_eq!(s, r#""voz:\"q\\e\"\u000a""#);
    }
}
