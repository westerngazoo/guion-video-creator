//! R-0019 AC9, AC10: the accents reach the phonemes.
//!
//! A battery of Spanish words goes through the exact phonemization path of
//! synthesis (the `piper` executable, whose `--debug` log prints the phonemes
//! it speaks), and the primary stress mark `ˈ` must land on the syllable the
//! spelling asks for. The live test needs Piper and a Spanish voice:
//!
//! ```sh
//! GUION_PIPER=~/.rotorfisico/piper312/bin/piper \
//! GUION_VOZ_ES=~/.rotorfisico/voces/es-carlfm-x-low/es-carlfm-x-low.onnx \
//!   cargo test -p guion-audio --test acentos -- --ignored --nocapture
//! ```
//!
//! In CI, the same check runs on the phonemes piper-tts 1.8.0 printed for the
//! battery with `es-carlfm-x-low` (espeak-ng voice `es`) on 2026-11-26.

use guion_audio::{compose, Lexicon, PiperEngine, Prosody};

/// (word, the stressed syllable counted from the end: 1 aguda, 2 grave,
/// 3 esdrújula, 4 sobresdrújula).
const BATTERY: &[(&str, usize)] = &[
    // agudas con tilde
    ("canción", 1),
    ("camión", 1),
    ("después", 1),
    ("también", 1),
    ("publicó", 1),
    ("ñandú", 1),
    ("dieciséis", 1),
    ("Raúl", 1),
    ("país", 1),
    // graves con tilde, y sin ella
    ("árbol", 2),
    ("lápiz", 2),
    ("fácil", 2),
    ("difícil", 2),
    ("fútbol", 2),
    ("azúcar", 2),
    ("oído", 2),
    ("publico", 2),
    ("pingüino", 2),
    ("cigüeña", 2),
    ("vergüenza", 2),
    ("niño", 2),
    ("España", 2),
    // esdrújulas y una sobresdrújula
    ("público", 3),
    ("teléfono", 3),
    ("sábado", 3),
    ("máquina", 3),
    ("música", 3),
    ("física", 3),
    ("kilómetros", 3),
    ("rápido", 3),
    ("último", 3),
    ("esdrújula", 3),
    ("dígamelo", 4),
];

/// What piper-tts 1.8.0 printed for [`BATTERY`], word by word, with
/// `es-carlfm-x-low` (2026-11-26).
const RECORDED: &[&str] = &[
    "kanθjˈon.",
    "kamjˈon.",
    "despwˈes.",
    "tambjˈen.",
    "pˌuβlikˈo.",
    "ɲandˈu.",
    "djˌeθisˈeis.",
    "raˈul.",
    "paˈis.",
    "ˈaɾβol.",
    "lˈapiθ.",
    "fˈaθil.",
    "difˈiθil.",
    "fˈutβol.",
    "aθˈukaɾ.",
    "oˈiðo.",
    "puβlˈiko.",
    "pˌiŋɡuˈino.",
    "θˌiɣuˈeɲa.",
    "bˌeɾɣuˈɛnθa.",
    "nˈiɲo.",
    "espˈaɲa.",
    "pˈuβliko.",
    "telˈefono.",
    "sˈaβaðo.",
    "mˈakina.",
    "mˈusika.",
    "fˈisika.",
    "kilˈometɾos.",
    "rˈapiðo.",
    "ˈultimo.",
    "esðɾˈuxula.",
    "dˈiɣamˌelo.",
];

/// The vowels espeak-ng's Spanish (and its neighbours) write.
const VOWELS: &[char] = &[
    'a', 'e', 'i', 'o', 'u', 'ɛ', 'ɔ', 'ə', 'ɪ', 'ʊ', 'æ', 'ɐ', 'ɑ', 'y', 'ø', 'œ', 'ɨ',
];

/// The syllable carrying the primary stress `ˈ`, counted from the end of
/// the word (a syllable is a run of vowels; `ˈ` between two vowels, as in a
/// hiatus, splits them). `None`: no primary stress.
fn stress_from_end(word: &str) -> Option<usize> {
    let i = word.find('ˈ')?;
    let mut groups = 0;
    let mut in_vowel = false;
    for c in word[i..].chars() {
        let v = VOWELS.contains(&c);
        if v && !in_vowel {
            groups += 1;
        }
        in_vowel = v;
    }
    Some(groups)
}

/// The words of a sentence's phonemes, without punctuation.
fn words(sentence: &str) -> Vec<String> {
    sentence
        .split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| !matches!(c, '.' | ',' | '?' | '!' | '¿' | '¡' | ';' | ':'))
                .collect()
        })
        .collect()
}

fn check_battery(phonemes: &[String]) {
    assert_eq!(phonemes.len(), BATTERY.len(), "one sentence per word");
    let mut wrong = Vec::new();
    for ((word, expected), ph) in BATTERY.iter().zip(phonemes) {
        let got = stress_from_end(&words(ph)[0]);
        if got != Some(*expected) {
            wrong.push(format!("{word} → {ph}: {got:?}, expected {expected}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "stress in the wrong place:\n{}",
        wrong.join("\n")
    );
}

/// AC9 (CI): the stress of every word of the battery, on recorded phonemes.
#[test]
fn ac9_the_recorded_battery_carries_each_stress() {
    let recorded: Vec<String> = RECORDED.iter().map(|s| s.to_string()).collect();
    check_battery(&recorded);
    // the reader itself: a hiatus, a diphthong, a sobresdrújula, no stress
    assert_eq!(stress_from_end("oˈiðo"), Some(2));
    assert_eq!(stress_from_end("djˌeθisˈeis"), Some(1));
    assert_eq!(stress_from_end("dˈiɣamˌelo"), Some(4));
    assert_eq!(stress_from_end("kˌomo"), None);
}

/// AC9, AC10, AC11 (live): the battery through Piper; the pairs in context
/// (the stressed one has a primary stress, the other none); ñ and ü; NFD
/// gives the phonemes of NFC; and the starter lexicon unstresses «mas»,
/// which espeak-ng stresses like «más».
#[test]
#[ignore = "needs Piper and a Spanish voice: GUION_PIPER, GUION_VOZ_ES"]
fn ac9_live_the_accents_reach_the_phonemes() {
    let piper = std::env::var("GUION_PIPER").expect("GUION_PIPER: the piper executable");
    let voice = std::env::var("GUION_VOZ_ES").expect("GUION_VOZ_ES: a Spanish .onnx");
    let e = PiperEngine::with_prosody(voice, Prosody::NATURAL).binary(piper);

    // capitalized: espeak-ng does not end a sentence at a «.» before a
    // lowercase word (it takes it for an abbreviation)
    let text: String = BATTERY
        .iter()
        .map(|(w, _)| {
            let mut c = w.chars();
            let first: String = c.next().unwrap().to_uppercase().collect();
            format!("{first}{}. ", c.as_str())
        })
        .collect();
    let ph = e.phonemes(text.trim()).unwrap();
    for ((w, _), p) in BATTERY.iter().zip(&ph) {
        println!("{w:>12}  {p}");
    }
    check_battery(&ph);

    // AC10: the same text in NFD (base letter + combining mark) says the same
    let nfd: String = text
        .chars()
        .flat_map(|c| match c {
            'á' => vec!['a', '\u{301}'],
            'é' => vec!['e', '\u{301}'],
            'í' => vec!['i', '\u{301}'],
            'ó' => vec!['o', '\u{301}'],
            'ú' => vec!['u', '\u{301}'],
            'ñ' => vec!['n', '\u{303}'],
            'ü' => vec!['u', '\u{308}'],
            'Á' => vec!['A', '\u{301}'],
            'Ú' => vec!['U', '\u{301}'],
            'Ñ' => vec!['N', '\u{303}'],
            c => vec![c],
        })
        .collect();
    assert_ne!(nfd, text);
    assert_eq!(compose(&nfd), text);
    assert_eq!(e.phonemes(nfd.trim()).unwrap(), ph, "NFD speaks as NFC");

    // the pairs, in context
    let lex = Lexicon::starter_es();
    let says = |t: &str| e.phonemes(&lex.apply(t)).unwrap();
    let ctx = says("¿Cómo lo haces? Lo hago como tú. Más o menos, mas no siempre.");
    println!("{ctx:?}");
    assert_eq!(ctx.len(), 3, "the lexicon moves no sentence cut: {ctx:?}");
    let primary = |s: usize, w: usize| words(&ctx[s])[w].contains('ˈ');
    assert!(primary(0, 0), "«cómo» is stressed: {ctx:?}");
    assert!(!primary(1, 2), "«como» is not: {ctx:?}");
    assert!(primary(2, 0), "«más» is stressed: {ctx:?}");
    assert!(!primary(2, 3), "«mas» is not (starter lexicon): {ctx:?}");
    let raw = e.phonemes("Más o menos, mas no siempre.").unwrap();
    assert!(
        words(&raw[0])[3].contains('ˈ'),
        "without the lexicon espeak-ng stresses «mas»: {raw:?}"
    );
    let trio = says("El público publicó lo que yo publico.");
    let w = words(&trio[0]);
    assert_eq!(
        [1, 2, 6].map(|i| stress_from_end(&w[i])),
        [Some(3), Some(1), Some(2)],
        "{trio:?}"
    );

    // ñ is /ɲ/; ü is a /u/ that «gui» lacks
    let s = says("Niño. Pingüino. Guiso.");
    assert!(s[0].contains('ɲ'), "{s:?}");
    assert!(s[1].contains("ɡu"), "{s:?}");
    assert!(!s[2].contains('u'), "{s:?}");
}
