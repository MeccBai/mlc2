//! Cache placement and publication fallback, independent of compilation.
use crate::artifacts::{self, Artifact, ArtifactPaths, GenericBundle, Manifest};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn writable_stem(stems: &[PathBuf]) -> Result<(PathBuf, Vec<String>), String> {
    let mut warnings = vec![];
    for stem in stems {
        let root = stem.parent().ok_or("Artifact has no parent")?;
        match fs::create_dir_all(root).and_then(|_| tempfile::NamedTempFile::new_in(root).map(drop))
        {
            Ok(()) => return Ok((stem.clone(), warnings)),
            Err(error) => warnings.push(format!(
                "Cache directory {} is not writable: {error}",
                root.display()
            )),
        }
    }
    Err(warnings.join("\n"))
}

pub(super) fn publish(
    stems: &[PathBuf],
    chosen: &Path,
    manifest: Manifest,
    templates: Option<GenericBundle>,
    object: Option<&[u8]>,
    target: &str,
) -> Result<Artifact, String> {
    let mut warnings = vec![];
    for stem in stems.iter().skip_while(|stem| stem.as_path() != chosen) {
        match artifacts::publish(
            &ArtifactPaths::new(stem, target),
            manifest.clone(),
            templates.clone(),
            object,
        ) {
            Ok(mut artifact) => {
                artifact.warnings = warnings;
                return Ok(artifact);
            }
            Err(error) => warnings.push(format!(
                "Cache publication failed at {}: {error}",
                stem.display()
            )),
        }
    }
    Err(warnings.join("\n"))
}
