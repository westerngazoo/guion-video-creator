# SPEC-0010 — Instagram export

- **Status:** In review
- **Realizes:** R-0010
- **Created:** 2026-10-14
- **Module(s):** `guion-publish` (new crate)

## 1. Motivation

Realizes R-0010. Lifted from rotorf-sico's `tools/herramientas` (`src/bin/caption.rs`
and the Python-compatible text helpers of its `lib.rs`), operation for
operation, so the two can be compared with `diff`.

## 2. Design

```
guion-publish/
├── src/lib.rs          # re-exports
├── src/instagram.rs    # export, export_file, Export, sections, hashtags, plain, the limits
├── src/text.rs         # is_word, is_space, strip, splitlines, read_text (Python's classes)
├── src/py_unicode.rs   # generated: WORD, SPACE, LINEBREAK
├── src/error.rs        # PublishError
├── src/bin/caption.rs  # caption <caption.md> <out dir>
└── tools/gen_tables.py # regenerates py_unicode.rs (Python 3.11.15, Unicode 14.0.0)
```

### Use

```rust
let e = guion_publish::export_file(Path::new("posts/x/caption.md"))?; // checks AC2–AC4
let (caption_txt, alt_txt) = e.write_to(Path::new("publish/x"))?;     // AC5
```

```bash
cargo run --release -p guion-publish --bin caption -- posts/x/caption.md publish/x
```

### The export (AC1–AC4)

1. `read_text`: UTF-8 (else `NotUtf8` with the byte offset), universal newlines.
2. `sections`: for each `splitlines` line, `## <name>` (a non-empty rest)
   opens section `strip(name).to_lowercase()`; other lines go to the open
   section; each section's text is `strip(lines.join("\n"))`.
3. Hashtags: `#` then one or more `\w` (Python's), over
   `"{hashtags} {caption}"`; more than `MAX_HASHTAGS` (5) fails.
4. `plain`: split on `\n[ \t]*\n`; drop paragraphs that are all `isspace`;
   collapse `[ \t\r\n]+` to one space and strip; remove the marks of the
   first `**…**` (non-empty, no newline) and of `_…_` not touching word
   characters on the outside; join with `"\n\n"`.
5. The pasted caption is `plain(caption) + "\n\n" + plain(hashtags)`; more
   than `MAX_CAPTION_CHARS` (2 200) code points fails.

All checks run before any file is written (AC5).

## 3. Acceptance-criteria checklist

| AC | Test |
|----|------|
| AC1 | `tests/r0010_instagram.rs::ac1_exports_the_two_texts`; unit `instagram::tests::{emphasis_as_re, paragraphs_as_re}`, `text::tests::splitlines_as_python` |
| AC2 | `ac2_at_most_five_hashtags` |
| AC3 | `ac3_at_most_2200_characters` |
| AC4 | `ac4_missing_sections_are_named` |
| AC5 | `ac5_writes_both_files_or_none` |
| AC6 | `text::tests::classes_as_python`; out of tree: the 20 `caption.md` of rotorf-sico, both binaries, `diff -r` of the outputs and the exit codes (2026-10-14: 20/20) |

## 4. Decision log

See R-0010 §6.

## Changelog

- 2026-10-14 — created with R-0010.
