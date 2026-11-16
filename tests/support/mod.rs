//! Isolated installation layout, rebuilt from tracked library inputs.
#![allow(dead_code)]
#[cfg(windows)]
mod windows;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};
#[cfg(windows)]
#[allow(unused_imports)]
pub use windows::{compile_c, link};

pub fn library() -> PathBuf {
    static INSTALLATION: OnceLock<tempfile::TempDir> = OnceLock::new();
    INSTALLATION
        .get_or_init(|| {
            let installation = tempfile::tempdir().unwrap();
            let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
            fn copy(source: &Path, destination: &Path) {
                fs::create_dir_all(destination).unwrap();
                for entry in fs::read_dir(source).unwrap() {
                    let entry = entry.unwrap();
                    if entry.file_name().to_string_lossy().starts_with('.') {
                        continue;
                    }
                    if entry.file_type().unwrap().is_dir() {
                        copy(&entry.path(), &destination.join(entry.file_name()));
                    } else {
                        fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
                    }
                }
            }
            copy(
                &repository.join("packages"),
                &installation.path().join("packages"),
            );
            let registry = mlc_builder::packages::Registry::load(installation.path()).unwrap();
            for triplet in [
                "i686-pc-windows-msvc",
                "x86_64-pc-windows-msvc",
                "x86_64-pc-windows-gnu",
                "x86_64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin",
            ] {
                for mode in [
                    mlc_builder::project::BuildMode::Debug,
                    mlc_builder::project::BuildMode::Release,
                ] {
                    for name in ["c_std", "std"] {
                        let requirement = mlc_builder::packages::Requirement {
                            name: name.into(),
                            version: env!("CARGO_PKG_VERSION").into(),
                            kind: mlc_builder::packages::PackageType::Source,
                            link_type: None,
                        };
                        for package in registry
                            .resolve(installation.path(), &[requirement], mode, triplet)
                            .unwrap()
                        {
                            mlc_builder::packages::prepare_declarations(&package, triplet).unwrap();
                        }
                    }
                }
            }
            installation
        })
        .path()
        .to_owned()
}

pub fn plan(entry: &Path) -> mlc_builder::plan::BuildPlan {
    let mut paths = mlc_builder::paths::PathResolver::standalone(
        &library().join("mlc.exe"),
        entry.parent().unwrap(),
        &mlc_builder::config::GlobalConfig::default(),
    )
    .unwrap();
    paths.package_root = library();
    paths.triplet = "x86_64-pc-windows-msvc".into();
    paths.resolve_packages().unwrap();
    mlc_builder::plan::BuildPlan::discover_with_paths(entry, &paths).unwrap()
}
