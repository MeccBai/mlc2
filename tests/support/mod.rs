//! Isolated installation layout, rebuilt from tracked library inputs.
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub fn library() -> PathBuf {
    static INSTALLATION: OnceLock<tempfile::TempDir> = OnceLock::new();
    INSTALLATION
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("lib");
            copy_inputs(&source, &source, root.path());
            root
        })
        .path()
        .join("lib")
}

fn copy_inputs(source: &Path, directory: &Path, installation: &Path) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            copy_inputs(source, &path, installation);
            continue;
        }
        let relative = path.strip_prefix(source).unwrap();
        match path.extension().and_then(|value| value.to_str()) {
            Some("m2") => {
                let destination = installation.join("lib").join(relative);
                fs::create_dir_all(destination.parent().unwrap()).unwrap();
                fs::copy(&path, destination).unwrap();
            }
            Some("toml") => {
                let destination = installation
                    .join("objects")
                    .join(relative)
                    .with_extension("sym");
                mlc_builder::artifacts::convert_toml(&path, &destination).unwrap();
            }
            _ => {}
        }
    }
}
