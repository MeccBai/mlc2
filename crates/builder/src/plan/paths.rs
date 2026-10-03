use mlc_syntax::{ImportModule, manifest::SOURCE_SUFFIX};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImportResolver {
    pub lib_dirs: Vec<PathBuf>,
}

impl ImportResolver {
    pub fn from_executable(executable: &Path) -> Result<Self, String> {
        let root = executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or("Executable has no installation root")?;
        Ok(Self {
            lib_dirs: vec![
                root.join(crate::manifest::LIB_DIR)
                    .join(crate::paths::HOST_TRIPLET),
            ],
        })
    }

    pub fn resolve(&self, importer: &Path, import: &ImportModule) -> Result<PathBuf, String> {
        if import.path().is_empty() {
            return Err("Empty import path".into());
        }
        let mut relative = PathBuf::new();
        for segment in import.path() {
            let path = Path::new(segment);
            if segment.is_empty()
                || path.components().count() != 1
                || !matches!(path.components().next(), Some(Component::Normal(_)))
                || segment.contains(['/', '\\', ':', '.'])
            {
                return Err(format!("Invalid import segment: {segment}"));
            }
            relative.push(segment);
        }
        let local = importer.parent().ok_or("Importing file has no directory")?;
        for root in std::iter::once(local).chain(self.lib_dirs.iter().map(PathBuf::as_path)) {
            // The directory priority is stronger than the source/declaration preference.
            for extension in [SOURCE_SUFFIX.trim_start_matches('.'), "sym"] {
                let candidate = root.join(&relative).with_extension(extension);
                if candidate.is_file() {
                    return candidate.canonicalize().map_err(|e| e.to_string());
                }
            }
        }
        // Exhaust target-specific roots first. Universal declarations never provide
        // target-specific objects, source code or generic templates.
        for root in &self.lib_dirs {
            let Some(parent) = root.parent() else {
                continue;
            };
            let candidate = parent
                .join(crate::manifest::UNIVERSAL_LIB_DIR)
                .join(&relative)
                .with_extension("sym");
            if candidate.is_file() {
                let artifact = crate::artifacts::load(&candidate)?;
                if artifact.manifest.config.object != crate::artifacts::ObjectMode::None {
                    return Err(format!(
                        "Universal library must use Object = None: {}",
                        candidate.display()
                    ));
                }
                return candidate.canonicalize().map_err(|e| e.to_string());
            }
        }
        Err(format!(
            "{}: import {} not found",
            importer.display(),
            import.path().join("::")
        ))
    }
}
