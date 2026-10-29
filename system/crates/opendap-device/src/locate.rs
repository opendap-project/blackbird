//! Where a Blackbird OS device's description is (hardware support spec §3): the one the hardware hands over at boot
//! (`/run`), else the one the image ships (`/usr/share`). One lookup for every program that reads it.

use std::path::{Path, PathBuf};

use crate::{Description, Error};

/// The places a description is looked for, first match wins.
pub const SEARCH_PATHS: &[&str] = &[
    opendap_platform::run::DEVICE_DESCRIPTION,
    "/usr/share/blackbird/device/description.toml",
];

/// The first of `paths` that exists.
pub fn find_in<P: AsRef<Path>>(paths: &[P]) -> Option<PathBuf> {
    paths
        .iter()
        .map(|p| p.as_ref().to_path_buf())
        .find(|p| p.exists())
}

/// The device's description file, if one exists.
pub fn find() -> Option<PathBuf> {
    find_in(SEARCH_PATHS)
}

/// Why a description could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("no device description in {0}")]
    NotFound(String),
    #[error("{path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Parse { path: PathBuf, source: Error },
}

/// Reads and parses the first description found among `paths`.
pub fn load_from<P: AsRef<Path>>(paths: &[P]) -> Result<(PathBuf, Description), LoadError> {
    let path = find_in(paths).ok_or_else(|| {
        LoadError::NotFound(
            paths
                .iter()
                .map(|p| p.as_ref().display().to_string())
                .collect::<Vec<_>>()
                .join(", "),
        )
    })?;
    let text = std::fs::read_to_string(&path).map_err(|source| LoadError::Read {
        path: path.clone(),
        source,
    })?;
    let d = Description::parse(&text).map_err(|source| LoadError::Parse {
        path: path.clone(),
        source,
    })?;
    Ok((path, d))
}

/// Reads and parses the device's description.
pub fn load() -> Result<(PathBuf, Description), LoadError> {
    load_from(SEARCH_PATHS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("opendap-locate-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const FIXTURE: &str = include_str!("../fixtures/virtual-t1.toml");

    #[test]
    fn the_run_description_wins_over_the_shipped_one() {
        let d = dir("first");
        let (run, usr) = (d.join("run.toml"), d.join("usr.toml"));
        std::fs::write(&run, FIXTURE).unwrap();
        std::fs::write(&usr, "not toml").unwrap();
        let (path, desc) = load_from(&[&run, &usr]).unwrap();
        assert_eq!(path, run);
        assert!(desc.display.is_some());
    }

    #[test]
    fn the_shipped_description_is_the_fallback() {
        let d = dir("fallback");
        let usr = d.join("usr.toml");
        std::fs::write(&usr, FIXTURE).unwrap();
        assert_eq!(find_in(&[d.join("absent.toml"), usr.clone()]), Some(usr));
    }

    #[test]
    fn none_found_and_a_bad_file_are_errors_that_name_the_path() {
        let d = dir("errors");
        assert!(matches!(
            load_from(&[d.join("absent.toml")]),
            Err(LoadError::NotFound(_))
        ));
        let bad = d.join("bad.toml");
        std::fs::write(&bad, "schema = 3").unwrap();
        let err = load_from(&[&bad]).unwrap_err();
        assert!(matches!(err, LoadError::Parse { .. }));
        assert!(err.to_string().contains("bad.toml"));
    }
}
