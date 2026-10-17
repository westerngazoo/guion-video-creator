//! R-0010 — the Instagram export: one test (or more) per acceptance criterion.

use guion_publish::instagram::{hashtags, plain, sections};
use guion_publish::{export, PublishError, MAX_CAPTION_CHARS, MAX_HASHTAGS};

const POST: &str = "# A post\n\n## Caption\n\nWhy does the kart\nlift a wheel?\n\n**Load transfer**: _ΔF = m a h / t_.\n\n## Hashtags\n\n#física #karting\n\n## Alt text\n\nA kart, seen from behind,\nlifts its inner wheel.\n";

/// AC1. The sections, flattened: one line per paragraph, no emphasis marks,
/// caption and hashtags separated by a blank line.
#[test]
fn ac1_exports_the_two_texts() {
    let e = export(POST, "caption.md").unwrap();
    assert_eq!(
        e.caption,
        "Why does the kart lift a wheel?\n\nLoad transfer: ΔF = m a h / t.\n\n#física #karting"
    );
    assert_eq!(
        e.alt_text,
        "A kart, seen from behind, lifts its inner wheel."
    );
    let s = sections("## Caption\na\n## caption\nb\n");
    assert_eq!(s["caption"], "b", "a repeated section starts over");
    assert_eq!(plain("a\r\nb"), "a b");
}

/// AC2. More than five hashtags, in the caption and the hashtags together, fails.
#[test]
fn ac2_at_most_five_hashtags() {
    assert_eq!(MAX_HASHTAGS, 5);
    assert_eq!(
        hashtags("#física #k_2 # #-x ##y"),
        vec!["#física", "#k_2", "#y"]
    );
    let five = POST.replace("#física #karting", "#a #b #c #d #e");
    assert!(export(&five, "x").is_ok());
    let six = POST
        .replace("lift a wheel?", "lift a wheel? #f")
        .replace("#física #karting", "#a #b #c #d #e");
    let e = export(&six, "posts/x/caption.md").unwrap_err();
    assert!(
        matches!(&e, PublishError::TooManyHashtags { count: 6, .. }),
        "{e}"
    );
    assert!(
        e.to_string().starts_with("posts/x/caption.md: 6 hashtags"),
        "{e}"
    );
}

/// AC3. The pasted caption, counted in code points, may not pass 2 200.
#[test]
fn ac3_at_most_2200_characters() {
    assert_eq!(MAX_CAPTION_CHARS, 2200);
    // "## Hashtags" gives "#a" (2) and the separator "\n\n" (2): 2 196 of caption fit exactly
    let fits = format!(
        "## Caption\n{}\n## Hashtags\n#a\n## Alt text\nx\n",
        "é".repeat(2196)
    );
    assert_eq!(export(&fits, "x").unwrap().caption.chars().count(), 2200);
    let long = fits.replacen('é', "éé", 1);
    assert!(matches!(
        export(&long, "x"),
        Err(PublishError::CaptionTooLong { chars: 2201, .. })
    ));
}

/// AC4. A missing section fails, naming it and the file.
#[test]
fn ac4_missing_sections_are_named() {
    let e = export("## Caption\na\n## Hashtags\n#a\n", "p/caption.md").unwrap_err();
    assert_eq!(
        e.to_string(),
        "p/caption.md: the section «## alt text» is missing"
    );
}

/// AC5. Nothing is written when the export fails; on success both files end in a newline.
#[test]
fn ac5_writes_both_files_or_none() {
    let dir = std::env::temp_dir().join(format!("guion-publish-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let md = dir.join("caption.md");
    std::fs::write(&md, POST.replace("#física #karting", "#a #b #c #d #e #f")).unwrap();
    assert!(guion_publish::export_file(&md).is_err());
    assert!(!dir.join("caption.txt").exists() && !dir.join("alt-text.txt").exists());
    std::fs::write(&md, POST.replace('\n', "\r\n")).unwrap();
    let (c, a) = guion_publish::export_file(&md)
        .unwrap()
        .write_to(&dir)
        .unwrap();
    assert!(std::fs::read_to_string(c).unwrap().ends_with("#karting\n"));
    assert!(std::fs::read_to_string(a).unwrap().ends_with("wheel.\n"));
}
