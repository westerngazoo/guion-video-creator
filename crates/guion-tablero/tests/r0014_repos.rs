//! R-0014 AC8 — the dashboard across repos: repeatable `--repo`, the user's
//! list of repos (`--todas`, or the default where the current folder
//! declares no accounts), and one state with every account of every repo.

use std::path::{Path, PathBuf};

use guion_tablero::repos;
use guion_tablero::tablero::Tablero;

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("guion-repos-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A repo with one account and one post.
fn repo(base: &Path, name: &str, cuenta: &str) -> PathBuf {
    let r = base.join(name);
    std::fs::create_dir_all(r.join("posts/uno")).unwrap();
    std::fs::write(
        r.join("guion-cuentas.toml"),
        format!("[[cuenta]]\nnombre = \"{cuenta}\"\nposts = [\"posts/*\"]\n"),
    )
    .unwrap();
    std::fs::write(
        r.join("posts/uno/README.md"),
        "# Uno\n\n- **estado:** listo\n",
    )
    .unwrap();
    r
}

#[test]
fn ac8_repo_repeats_and_the_same_repo_is_read_once() {
    let args: Vec<String> = ["--repo", "a", "-r", "b", "--repo", "a", "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let o = guion_tablero::opciones(&args).unwrap();
    assert_eq!(o.workspaces, [PathBuf::from("a"), PathBuf::from("b")]);
    assert!(o.json && !o.todas);
    assert!(guion_tablero::opciones(&["--repo".into()])
        .unwrap_err()
        .contains("--repo"));
}

#[test]
fn ac8_which_repos_when_none_is_named() {
    let d = dir("elegir");
    let rotor = repo(&d, "rotorf-sico", "Rotor Físico");
    let fbf = repo(&d, "fisicobuenfisico", "fisicobuenfisico");
    let vacia = d.join("vacia");
    std::fs::create_dir_all(&vacia).unwrap();
    let lista = vec![rotor.clone(), fbf.clone()];
    // a folder that declares accounts is read alone…
    assert_eq!(
        repos::elegir(&[], false, &rotor, Some(lista.clone())),
        vec![rotor.clone()]
    );
    // …one that does not falls back to the user's list…
    assert_eq!(
        repos::elegir(&[], false, &vacia, Some(lista.clone())),
        lista
    );
    // …and with no list, it is itself (its missing file is then the error)
    assert_eq!(repos::elegir(&[], false, &vacia, None), vec![vacia.clone()]);
    // --todas adds the list to what was named, each repo once
    assert_eq!(
        repos::elegir(
            std::slice::from_ref(&fbf),
            true,
            &vacia,
            Some(lista.clone())
        ),
        [fbf.clone(), rotor.clone()]
    );
    // the same repo by two spellings is one
    let otra = rotor.join("..").join("rotorf-sico");
    assert_eq!(
        repos::elegir(&[rotor.clone(), otra], false, &vacia, None),
        [rotor]
    );
}

#[test]
fn ac8_the_user_list_grows_by_appending_and_shows_every_account() {
    let d = dir("lista");
    let rotor = repo(&d, "rotorf-sico", "Rotor Físico");
    let fbf = repo(&d, "fisicobuenfisico", "fisicobuenfisico");
    let f = d.join("config/guion/repos.toml");
    assert!(repos::agregar(&f, &rotor).unwrap());
    let antes = std::fs::read_to_string(&f).unwrap();
    assert!(repos::agregar(&f, &fbf).unwrap());
    assert!(!repos::agregar(&f, &fbf).unwrap(), "already there");
    let ahora = std::fs::read_to_string(&f).unwrap();
    assert!(ahora.starts_with(&antes), "appended, never rewritten");
    let leidos = repos::leer(&f).unwrap();
    assert_eq!(leidos.len(), 2);
    let t = Tablero::leer(&leidos, &|_| None).unwrap();
    let nombres: Vec<&str> = t.cuentas.iter().map(|c| c.nombre.as_str()).collect();
    assert_eq!(nombres, ["Rotor Físico", "fisicobuenfisico"]);
    assert!(t.cuentas.iter().all(|c| c.conteo.listo == 1));
    // a relative entry is relative to the list's folder
    std::fs::write(&f, "[[repo]]\nruta = \"../../fisicobuenfisico\"\n").unwrap();
    assert!(repos::mismo(&repos::leer(&f).unwrap()[0], &fbf));
}
