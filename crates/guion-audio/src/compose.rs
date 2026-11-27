//! Composed text for the voice (R-0019 AC10).
//!
//! espeak-ng reads a precomposed «ú» (U+00FA) as a stressed vowel, but «u»
//! followed by a combining acute (U+0301), the NFD spelling that macOS file
//! names and some copy-paste sources use, as a plain «u» and an unknown mark:
//! measured through Piper 1.8.0, «público» became *publíco* and «física»
//! *fisíca*. [`compose`] turns those pairs back into the precomposed letters.
//!
//! It is canonical composition (NFC) for the letters Spanish uses and their
//! neighbours (the Latin vowels, `n`, `c`, `y` with grave, acute, circumflex,
//! tilde, diaeresis or cedilla), not the whole Unicode algorithm: a text with
//! other combining marks passes as it is. Composed text comes back unchanged,
//! so a voice that already got NFC keeps its bytes. (The `unicode-normalization`
//! crate would cover every script; a table of the letters a Spanish voice
//! reads needs no dependency.)

/// The precomposed letter for `base` followed by the combining `mark`.
fn pair(base: char, mark: char) -> Option<char> {
    // each row: the combining mark, then (base, composed) pairs
    const TABLE: &[(char, &[(char, char)])] = &[
        (
            '\u{300}', // grave
            &[
                ('a', 'à'),
                ('e', 'è'),
                ('i', 'ì'),
                ('o', 'ò'),
                ('u', 'ù'),
                ('A', 'À'),
                ('E', 'È'),
                ('I', 'Ì'),
                ('O', 'Ò'),
                ('U', 'Ù'),
            ],
        ),
        (
            '\u{301}', // acute
            &[
                ('a', 'á'),
                ('e', 'é'),
                ('i', 'í'),
                ('o', 'ó'),
                ('u', 'ú'),
                ('y', 'ý'),
                ('A', 'Á'),
                ('E', 'É'),
                ('I', 'Í'),
                ('O', 'Ó'),
                ('U', 'Ú'),
                ('Y', 'Ý'),
            ],
        ),
        (
            '\u{302}', // circumflex
            &[
                ('a', 'â'),
                ('e', 'ê'),
                ('i', 'î'),
                ('o', 'ô'),
                ('u', 'û'),
                ('A', 'Â'),
                ('E', 'Ê'),
                ('I', 'Î'),
                ('O', 'Ô'),
                ('U', 'Û'),
            ],
        ),
        (
            '\u{303}', // tilde
            &[
                ('a', 'ã'),
                ('n', 'ñ'),
                ('o', 'õ'),
                ('A', 'Ã'),
                ('N', 'Ñ'),
                ('O', 'Õ'),
            ],
        ),
        (
            '\u{308}', // diaeresis
            &[
                ('a', 'ä'),
                ('e', 'ë'),
                ('i', 'ï'),
                ('o', 'ö'),
                ('u', 'ü'),
                ('y', 'ÿ'),
                ('A', 'Ä'),
                ('E', 'Ë'),
                ('I', 'Ï'),
                ('O', 'Ö'),
                ('U', 'Ü'),
            ],
        ),
        ('\u{327}', &[('c', 'ç'), ('C', 'Ç')]), // cedilla
    ];
    TABLE
        .iter()
        .find(|(m, _)| *m == mark)?
        .1
        .iter()
        .find(|(b, _)| *b == base)
        .map(|(_, c)| *c)
}

/// `text` with every Latin letter + combining mark of the table composed.
pub fn compose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match chars.peek().and_then(|&m| pair(c, m)) {
            Some(composed) => {
                out.push(composed);
                chars.next();
            }
            None => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::compose;

    #[test]
    fn nfd_spanish_composes_and_nfc_passes() {
        // the NFD spellings, written out
        let nfd = "pu\u{301}blico publico\u{301} nin\u{303}o pingu\u{308}ino A\u{301}REA \
                   Nin\u{303}a CAMIO\u{301}N c\u{327}a";
        assert_eq!(
            compose(nfd),
            "público publicó niño pingüino ÁREA Niña CAMIÓN ça"
        );
        let nfc = "El público publicó lo que yo publico: ñandú, cigüeña, «sí».";
        assert_eq!(compose(nfc), nfc);
        // a mark with no letter of the table stays as it is
        assert_eq!(compose("x\u{301} \u{301}a"), "x\u{301} \u{301}a");
    }
}
