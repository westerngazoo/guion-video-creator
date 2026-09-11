use std::path::{Path, PathBuf};

/// Resolve a screenplay path: cwd-relative first, then workspace `guion/` root.
///
/// Lets `guion check templates/foo.toml` work even when invoked from `target/`.
pub fn resolve_screenplay_path(path: &Path) -> PathBuf {
    if path.is_absolute() && path.exists() {
        return path.to_path_buf();
    }
    if !path.is_relative() {
        return path.to_path_buf();
    }
    if path.exists() {
        return path.to_path_buf();
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let candidate = workspace.join(path);
    if candidate.exists() {
        return candidate;
    }

    path.to_path_buf()
}

pub fn screenplay_not_found(path: &Path, resolved: &Path) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "?".into());
    format!(
        "guion: no encuentra el screenplay: {}\n\
         cwd: {}\n\
         probado también: {}\n\
         tip: corre desde la raíz del workspace `guion/`, o pasa una ruta absoluta",
        path.display(),
        cwd,
        resolved.display(),
    )
}
