//! Content-validated caches for supplemental objects and final link products.
use mlc_builder::artifacts::content_hash;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn sidecar(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".cache");
    PathBuf::from(name)
}

/// Include tool content and search environment, not just the command spelling.
pub(super) fn tool_identity(command: &Path) -> Option<(PathBuf, String)> {
    let mut candidates = vec![command.to_owned()];
    if command.components().count() == 1 {
        if let Some(search) = std::env::var_os("PATH") {
            for root in std::env::split_paths(&search) {
                candidates.push(root.join(command));
                #[cfg(windows)]
                if command.extension().is_none() {
                    for extension in std::env::var("PATHEXT")
                        .unwrap_or_else(|_| ".EXE;.CMD;.BAT".into())
                        .split(';')
                    {
                        let mut name = command.as_os_str().to_owned();
                        name.push(extension);
                        candidates.push(root.join(name));
                    }
                }
            }
        }
    }
    candidates.into_iter().find_map(|path| {
        let path = path.canonicalize().ok()?;
        let bytes = fs::read(&path).ok()?;
        Some((path, content_hash(&bytes)))
    })
}

pub(super) fn matches(path: &Path, key: &str) -> bool {
    let Ok(record) = fs::read_to_string(sidecar(path)) else {
        return false;
    };
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    record == format!("{key}\n{}\n", content_hash(&bytes))
}

pub(super) fn record(path: &Path, key: &str) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let parent = path.parent().ok_or("Cache output has no parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
    writeln!(file, "{key}\n{}", content_hash(&bytes)).map_err(|error| error.to_string())?;
    file.as_file()
        .sync_all()
        .map_err(|error| error.to_string())?;
    file.persist(sidecar(path))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_validates_inputs_and_output_contents() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("main.obj");
        assert!(!matches(&output, "key"));
        fs::write(&output, b"object").unwrap();
        record(&output, "key").unwrap();
        assert!(matches(&output, "key"));
        assert!(!matches(&output, "changed-input"));
        fs::write(&output, b"modified").unwrap();
        assert!(!matches(&output, "key"));
        record(&output, "key").unwrap();
        fs::write(sidecar(&output), "broken").unwrap();
        assert!(!matches(&output, "key"));
    }
}
