//! A pronunciation lexicon: how the voice says a written term (R-0019 AC11).
//!
//! espeak-ng phonemizes everything with the voice's language, so «byte» is
//! read as Spanish *bite*, «IA» as *ya* and «N·m» as *ene punto medio eme*.
//! A lexicon rewrites such terms in the text the voice says; what is written
//! (subtitles, `voz.json`) keeps the original.
//!
//! Matching: a term matches where it stands as a whole word (no letter or
//! digit right before or after it), longest term first, left to right, in
//! one pass (a replacement is never matched again). A term in lowercase also
//! matches with its first letter capitalized, as at the start of a sentence.

use std::collections::BTreeMap;

/// Terms and what the voice says instead.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lexicon {
    entries: BTreeMap<String, String>,
}

impl Lexicon {
    pub fn new() -> Self {
        Lexicon::default()
    }

    /// Adds (or replaces) a term. `Err` when the term or the replacement is
    /// empty, or either holds `.`, `?` or `!`: the voice text must cut into
    /// the same sentences as the written one (R-0019 AC11).
    pub fn insert(&mut self, term: &str, say: &str) -> Result<(), String> {
        let term = term.trim();
        if term.is_empty() || say.trim().is_empty() {
            return Err(format!("«{term}» = «{say}»: empty"));
        }
        for t in [term, say] {
            if t.contains(['.', '?', '!']) {
                return Err(format!(
                    "«{term}» = «{say}»: no . ? ! (they would move a sentence cut)"
                ));
            }
        }
        self.entries.insert(term.to_string(), say.to_string());
        Ok(())
    }

    /// Every entry of `other`, over this one's.
    pub fn extend(&mut self, other: &Lexicon) {
        for (k, v) in &other.entries {
            self.entries.insert(k.clone(), v.clone());
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// `text` as the voice says it.
    pub fn apply(&self, text: &str) -> String {
        if self.entries.is_empty() {
            return text.to_string();
        }
        // longest first: «km/h» before «km», «hip thrust» before «hip»
        let mut terms: Vec<(Vec<char>, &str)> = Vec::new();
        for (k, v) in &self.entries {
            terms.push((k.chars().collect(), v.as_str()));
            // not for Piper's raw phonemes: at the start of a sentence, piper-tts
            // 1.8.0 glues a `[[…]]` block to the sentence before (measured), and
            // the sentence cut would be lost
            if let Some(cap) = capitalized(k).filter(|_| !v.starts_with("[[")) {
                if !self.entries.contains_key(&cap) {
                    terms.push((cap.chars().collect(), v.as_str()));
                }
            }
        }
        terms.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(a.0.cmp(&b.0)));
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < chars.len() {
            let before_ok = i == 0 || !chars[i - 1].is_alphanumeric();
            let hit = before_ok
                .then(|| {
                    terms.iter().find(|(t, _)| {
                        chars[i..].starts_with(t)
                            && chars.get(i + t.len()).is_none_or(|c| !c.is_alphanumeric())
                    })
                })
                .flatten();
            match hit {
                Some((t, say)) => {
                    out.push_str(say);
                    i += t.len();
                }
                None => {
                    out.push(chars[i]);
                    i += 1;
                }
            }
        }
        out
    }
}

/// The starter list (`"guion:es"` in `voz.toml`): terms of the house's
/// subjects (units, tech, racing, the gym) that espeak-ng's Spanish misreads,
/// each replacement checked through Piper 1.8.0 on 2026-11-26 (the misreading
/// in the comment).
const STARTER_ES: &[(&str, &str)] = &[
    // units and symbols
    ("N·m", "newton metro"),         // ene punto medio eme
    ("km/h", "kilómetros por hora"), // ka eme barra hache
    ("kg", "kilos"),                 // kilogramo, in the singular
    ("hp", "caballos de fuerza"),    // ache pe
    // tech
    ("IA", "i a"),                         // ya
    ("CRM", "ce erre eme"),                // (the owner's example)
    ("API", "a pe i"),                     // ápi
    ("byte", "bait"),                      // bite
    ("bytes", "baits"),                    // bites
    ("software", "sóftuer"),               // softwér
    ("hardware", "járduer"),               // hardwér, with an h
    ("backprop", "bácprop"),               // backprop, stressed last
    ("machine learning", "machín lérnin"), // machíne leárning
    ("deep learning", "dip lérnin"),       // deép leárning
    ("online", "onláin"),                  // onlíne
    ("router", "ráuter"),                  // routér
    ("wifi", "uaifai"),                    // wífi
    ("smartphone", "smártfon"),            // smartphóne
    ("chatbot", "chátbot"),                // chatbót
    // racing
    ("downforce", "dáunfors"), // downfórce
    ("setup", "sétap"),        // setúp
    ("pole", "poul"),          // pole, as Spanish
    ("pitlane", "pitléin"),    // pitláne
    // the gym
    ("press", "pres"),           // press, with a long s
    ("curl", "kerl"),            // curl, as Spanish
    ("hip thrust", "jip trast"), // ip thrust
    ("deadlift", "dédlift"),     // deadlíft
    ("squat", "skuat"),          // squat, as Spanish
    ("kettlebell", "kételbel"),  // kettlebéll
    ("crossover", "crosóver"),   // crossovér
    ("RIR", "erre i erre"),      // rir
    // espeak-ng stresses the conjunction «mas» like «más» (R-0019 AC9): Piper's
    // raw phonemes, unstressed. Inside a sentence only (see `apply`), and with
    // the PyPI build, which reads [[…]] (the C++ one does not)
    ("mas", "[[mas]]"),
];

impl Lexicon {
    /// The starter list for Spanish voices (R-0019 AC11).
    pub fn starter_es() -> Lexicon {
        let mut l = Lexicon::new();
        for (k, v) in STARTER_ES {
            l.insert(k, v).expect("the starter list holds no . ? !");
        }
        l
    }
}

/// `term` with its first letter uppercase, if it is all lowercase.
fn capitalized(term: &str) -> Option<String> {
    if term.chars().any(char::is_uppercase) {
        return None;
    }
    let mut c = term.chars();
    let first = c.next()?;
    let up: String = first.to_uppercase().collect();
    (up != first.to_string()).then(|| up + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(pairs: &[(&str, &str)]) -> Lexicon {
        let mut l = Lexicon::new();
        for (k, v) in pairs {
            l.insert(k, v).unwrap();
        }
        l
    }

    #[test]
    fn whole_words_longest_first_one_pass() {
        let l = lex(&[
            ("km", "kilómetros"),
            ("km/h", "kilómetros por hora"),
            ("N·m", "newton metro"),
            ("byte", "bait"),
            ("bait", "NO"),
        ]);
        assert_eq!(
            l.apply("A 100 km/h, 51 N·m; 3 km. Un byte, bytes, megabyte."),
            "A 100 kilómetros por hora, 51 newton metro; 3 kilómetros. Un bait, bytes, megabyte."
        );
        // a lowercase term also matches capitalized; an acronym only as written
        let l = lex(&[("software", "sóftuer"), ("IA", "i a")]);
        assert_eq!(
            l.apply("Software e IA, no ia ni Ia."),
            "sóftuer e i a, no ia ni Ia."
        );
    }

    #[test]
    fn the_starter_list_reads_the_house_terms() {
        let l = Lexicon::starter_es();
        assert_eq!(
            l.apply("Cincuenta y un N·m. La IA y el byte, mas no siempre; más bien un RIR."),
            "Cincuenta y un newton metro. La i a y el bait, [[mas]] no siempre; más bien un erre \
             i erre."
        );
        // raw phonemes never at a sentence start (Piper would glue it to the one before)
        assert_eq!(l.apply("Mas no siempre."), "Mas no siempre.");
    }

    #[test]
    fn a_term_that_would_move_a_sentence_cut_is_refused() {
        let mut l = Lexicon::new();
        assert!(l.insert("etc.", "etcétera").unwrap_err().contains(". ? !"));
        assert!(l.insert("ok", "¡okey!").is_err());
        assert!(l.insert("", "x").is_err());
        assert!(l.insert("x", " ").is_err());
        assert!(l.is_empty());
    }
}
