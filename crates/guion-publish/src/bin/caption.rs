//! `caption <caption.md> <out dir>`: writes `caption.txt` and `alt-text.txt`
//! for Instagram, or fails without writing anything (R-0010).

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [src, dst, ..] = args.as_slice() else {
        eprintln!("usage: caption <caption.md> <out dir>");
        return ExitCode::from(2);
    };
    match guion_publish::export_file(Path::new(src)).and_then(|e| e.write_to(Path::new(dst))) {
        Ok((caption, alt)) => {
            println!("ok {} {}", caption.display(), alt.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
