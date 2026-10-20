//! R-0016 — the YouTube text: title, description, chapters (AC5, AC6).

use guion_publish::youtube::{
    chapter_lines, chapters_in, check_description, check_title, export, parse, timestamp,
    validate_chapters, Capitulo, MAX_DESCRIPTION, MAX_TITLE_CHARS, MIN_CHAPTERS, MIN_CHAPTER_S,
};
use guion_publish::PublishError;

const MD: &str = "# post\n\n## Título\n\n¿Por qué el kart **levanta** una rueda?\n\n## Descripción\n\nLa transferencia de carga,\nen una curva.\n\n## Capítulos\n\n- inicio | La pregunta\n- gancho.fin | El modelo\n- beat-3 | La pista\n\n## Procedencia\n\nLas cifras son calculadas con el modelo del reel, no medidas.\n";

fn caps(starts: &[f64]) -> Vec<Capitulo> {
    starts
        .iter()
        .enumerate()
        .map(|(k, &s)| Capitulo {
            inicio: s,
            titulo: format!("Parte {}", k + 1),
        })
        .collect()
}

#[test]
fn ac6_parses_youtube_md_with_its_anchors() {
    let f = parse(MD, "youtube.md").unwrap();
    assert_eq!(f.titulo, "¿Por qué el kart levanta una rueda?");
    assert_eq!(
        f.capitulos,
        vec![
            ("inicio".to_string(), "La pregunta".to_string()),
            ("gancho.fin".to_string(), "El modelo".to_string()),
            ("beat-3".to_string(), "La pista".to_string()),
        ]
    );
    let bad = MD.replace("- beat-3 | La pista", "- beat-3 La pista");
    let e = parse(&bad, "youtube.md").unwrap_err().to_string();
    assert!(
        e.contains("beat-3 La pista") && e.contains("<anchor> | <title>"),
        "{e}"
    );
    let e = parse("## Título\nx\n## Descripción\ny\n", "y.md").unwrap_err();
    assert!(
        matches!(e, PublishError::MissingSection { ref section, .. } if section == "procedencia")
    );
}

#[test]
fn ac6_the_description_carries_chapters_and_provenance() {
    let f = parse(MD, "youtube.md").unwrap();
    let ex = export(&f, &caps(&[0.0, 12.7, 31.2]), 60.0, "youtube.md").unwrap();
    assert_eq!(
        ex.description,
        "La transferencia de carga, en una curva.\n\n00:00 Parte 1\n00:12 Parte 2\n00:31 Parte 3\n\nLas cifras son calculadas con el modelo del reel, no medidas."
    );
    // the chapters written are the chapters read back
    let read: Vec<u64> = chapters_in(&ex.description).iter().map(|c| c.0).collect();
    assert_eq!(read, vec![0, 12, 31]);
    // a Short: no chapters, the same text without them
    let short = export(&f, &[], 50.0, "youtube.md").unwrap();
    assert!(!short.description.contains("00:00"));
}

#[test]
fn ac5_chapters_follow_youtubes_rules_and_fail_naming_the_rule() {
    assert_eq!((MIN_CHAPTERS, MIN_CHAPTER_S), (3, 10));
    assert!(validate_chapters(&[0, 10, 20], 30.0).is_empty());
    let first = validate_chapters(&[1, 20, 40], 60.0);
    assert!(first.iter().any(|m| m.contains("not 00:00")), "{first:?}");
    let few = validate_chapters(&[0, 30], 60.0);
    assert!(few.iter().any(|m| m.contains("2 chapters")), "{few:?}");
    let short = validate_chapters(&[0, 9, 30], 60.0);
    assert!(short.iter().any(|m| m.contains("lasts 9 s")), "{short:?}");
    let order = validate_chapters(&[0, 30, 20], 60.0);
    assert!(
        order.iter().any(|m| m.contains("does not come after")),
        "{order:?}"
    );
    let tail = validate_chapters(&[0, 20, 55], 60.0);
    assert!(tail.iter().any(|m| m.contains("last chapter")), "{tail:?}");
    // the export refuses them and writes nothing
    let f = parse(MD, "youtube.md").unwrap();
    let e = export(&f, &caps(&[0.0, 5.0, 30.0]), 60.0, "youtube.md").unwrap_err();
    assert!(e.to_string().contains("lasts 5 s"), "{e}");
}

#[test]
fn ac6_title_and_description_limits() {
    assert_eq!((MAX_TITLE_CHARS, MAX_DESCRIPTION), (100, 5000));
    assert!(check_title(&"á".repeat(100)).is_empty());
    assert!(!check_title(&"a".repeat(101)).is_empty());
    assert!(!check_title("a <b>").is_empty());
    assert!(!check_title("  ").is_empty());
    assert!(check_description(&"a".repeat(5000)).is_empty());
    // 2 500 «ñ» are 2 500 characters and 5 000 bytes: fits; one more does not
    assert!(check_description(&"ñ".repeat(2500)).is_empty());
    assert!(!check_description(&"ñ".repeat(2501)).is_empty());
    assert!(!check_description("x > y").is_empty());
}

#[test]
fn ac5_timestamps_as_youtube_reads_them() {
    assert_eq!(timestamp(0, false), "00:00");
    assert_eq!(timestamp(75, false), "01:15");
    assert_eq!(timestamp(3723, true), "1:02:03");
    assert_eq!(
        chapter_lines(&caps(&[0.0, 3700.4]), 4000.0),
        "0:00:00 Parte 1\n1:01:40 Parte 2"
    );
    assert_eq!(
        chapters_in("intro\n00:00 a\n1:02:03 b\n7:61 no\n12:30\nx 00:10 y"),
        vec![(0, "a".to_string()), (3723, "b".to_string())]
    );
}
