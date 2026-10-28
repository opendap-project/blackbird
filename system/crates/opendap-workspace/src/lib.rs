//! Finding another repository's checkout, for build scripts that consume its published outputs
//! (RFC 0008): `ODAP_<NAME>_DIR` when set, else the workspace's `repos.tsv` above `start`.
//! A repository merged into the `blackbird` root (RFC 0017) is its folder there: in the same
//! checkout as `start`, else under `ODAP_BLACKBIRD_DIR` or the root's `repos.tsv` row.
//! Never a guessed relative path.

use std::path::{Path, PathBuf};

pub mod tokens;

/// The environment variable that points at repository `name`'s checkout.
pub fn env_var(name: &str) -> String {
    format!("ODAP_{}_DIR", name.to_uppercase().replace('-', "_"))
}

/// The repositories merged into a root (RFC 0017 §7), and the folder each landed in.
const MERGED: &[(&str, &str, &str)] = &[
    ("blackbird-os", "blackbird", "os"),
    ("blackbird-ui", "blackbird", "ui"),
    ("system", "blackbird", "system"),
    ("virtual-devices", "blackbird", "emulator"),
];

fn env_dir(var: &str) -> Option<Result<PathBuf, String>> {
    let v = std::env::var_os(var).filter(|v| !v.is_empty())?;
    let p = PathBuf::from(v);
    Some(if p.is_dir() {
        Ok(p)
    } else {
        Err(format!("{var}={} is not a directory", p.display()))
    })
}

/// The `repos.tsv` above `start` (its folder and text), if any.
fn workspace(start: &Path) -> Option<(PathBuf, String)> {
    start.ancestors().find_map(|dir| {
        let text = std::fs::read_to_string(dir.join("repos.tsv")).ok()?;
        Some((dir.to_path_buf(), text))
    })
}

fn row(ws: &Path, text: &str, name: &str) -> Option<PathBuf> {
    text.lines().skip(1).find_map(|line| {
        let mut cols = line.split('\t');
        let (path, slug) = (cols.next()?, cols.next()?);
        let dir = ws.join(path);
        (slug.trim().rsplit('/').next() == Some(name) && dir.is_dir()).then_some(dir)
    })
}

/// A merged repository's folder: in the checkout that holds `start`, else in its root's checkout.
fn merged(
    name: &str,
    start: &Path,
    ws: Option<&(PathBuf, String)>,
) -> Option<Result<PathBuf, String>> {
    let (_, root, folder) = MERGED.iter().find(|(n, _, _)| *n == name)?;
    let top = start.ancestors().find(|d| d.join(".git").exists());
    if let Some(dir) = top.map(|t| t.join(folder)).filter(|d| d.is_dir()) {
        return Some(Ok(dir));
    }
    let root_dir = match env_dir(&env_var(root)) {
        Some(r) => Some(r),
        None => ws.and_then(|(w, t)| row(w, t, root)).map(Ok),
    }?;
    Some(root_dir.map(|d| d.join(folder)).and_then(|d| {
        if d.is_dir() {
            Ok(d)
        } else {
            Err(format!("{} is not a directory", d.display()))
        }
    }))
}

/// The checkout of the GitHub repository `name` (for example "blackbird-ui").
pub fn repo_dir(name: &str, start: &Path) -> Result<PathBuf, String> {
    let var = env_var(name);
    if let Some(found) = env_dir(&var) {
        return found;
    }
    let ws = workspace(start);
    if let Some(dir) = ws.as_ref().and_then(|(w, t)| row(w, t, name)) {
        return Ok(dir);
    }
    if let Some(found) = merged(name, start, ws.as_ref()) {
        return found;
    }
    Err(match ws {
        Some((dir, _)) => format!(
            "{name} is not checked out in {}: run scripts/bootstrap.sh, or set {var}",
            dir.display()
        ),
        None => format!(
            "no workspace (repos.tsv) above {}: set {var}",
            start.display()
        ),
    })
}

/// For a build script: repository `name`'s checkout, after checking that each of its published `paths`
/// exists (`hint` says how to produce a missing one), and telling cargo to rebuild when the variable or
/// any of the paths changes.
pub fn published(name: &str, start: &Path, paths: &[&str], hint: &str) -> Result<PathBuf, String> {
    let root = repo_dir(name, start)?;
    println!("cargo:rerun-if-env-changed={}", env_var(name));
    for rel in paths {
        let p = root.join(rel);
        if !p.exists() {
            return Err(format!("{} not found: {hint}", p.display()));
        }
        println!("cargo:rerun-if-changed={}", p.display());
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_variable_after_the_repository() {
        assert_eq!(env_var("blackbird-ui"), "ODAP_BLACKBIRD_UI_DIR");
    }

    #[test]
    fn finds_a_repository_through_repos_tsv() {
        let root = std::env::temp_dir().join(format!("odap-ws-{}", std::process::id()));
        std::fs::create_dir_all(root.join("public/blackbird-ui")).unwrap();
        std::fs::create_dir_all(root.join("public/system/crates/x")).unwrap();
        std::fs::write(
            root.join("repos.tsv"),
            "path\trepository\npublic/blackbird-ui\topendap-project/blackbird-ui\n",
        )
        .unwrap();
        let found = repo_dir("blackbird-ui", &root.join("public/system/crates/x")).unwrap();
        assert_eq!(found, root.join("public/blackbird-ui"));
        assert!(repo_dir("nothing", &root.join("public/system")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_a_merged_repository_in_the_same_checkout() {
        let root = std::env::temp_dir().join(format!("odap-ws-same-{}", std::process::id()));
        std::fs::create_dir_all(root.join("blackbird/.git")).unwrap();
        std::fs::create_dir_all(root.join("blackbird/ui")).unwrap();
        std::fs::create_dir_all(root.join("blackbird/system/crates/x")).unwrap();
        let start = root.join("blackbird/system/crates/x");
        assert_eq!(
            repo_dir("blackbird-ui", &start).unwrap(),
            root.join("blackbird/ui")
        );
        assert!(repo_dir("blackbird-os", &start).is_err()); // no os/ folder: not guessed
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_a_merged_repository_through_its_root_row() {
        let root = std::env::temp_dir().join(format!("odap-ws-root-{}", std::process::id()));
        std::fs::create_dir_all(root.join("public/blackbird/ui")).unwrap();
        std::fs::create_dir_all(root.join("public/other")).unwrap();
        std::fs::write(
            root.join("repos.tsv"),
            "path\trepository\npublic/blackbird\topendap-project/blackbird\n",
        )
        .unwrap();
        let start = root.join("public/other");
        assert_eq!(
            repo_dir("blackbird-ui", &start).unwrap(),
            root.join("public/blackbird/ui")
        );
        assert!(repo_dir("nothing", &start).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn published_paths_must_exist() {
        let root = std::env::temp_dir().join(format!("odap-ws-pub-{}", std::process::id()));
        std::fs::create_dir_all(root.join("public/blackbird-ui/export")).unwrap();
        std::fs::create_dir_all(root.join("public/system")).unwrap();
        std::fs::write(
            root.join("repos.tsv"),
            "path\trepository\npublic/blackbird-ui\topendap-project/blackbird-ui\n",
        )
        .unwrap();
        let start = root.join("public/system");
        assert!(published("blackbird-ui", &start, &["export"], "run the generators").is_ok());
        let err = published(
            "blackbird-ui",
            &start,
            &["export/fonts"],
            "run the generators",
        )
        .unwrap_err();
        assert!(err.contains("export/fonts") && err.contains("run the generators"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
