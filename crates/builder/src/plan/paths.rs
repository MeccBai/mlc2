use mlc_manifest::SOURCE_SUFFIX;
use mlc_syntax::ImportModule;
use std::path::{Component, Path, PathBuf};
pub(super) enum ImportSearch {
    Found(PathBuf),
    Missing(Vec<PathBuf>),
}

impl ImportSearch {
    pub(super) fn require(self, importer: &Path, import: &ImportModule) -> Result<PathBuf, String> {
        match self {
            Self::Found(path) => Ok(path),
            Self::Missing(expected) => Err(format!(
                "{}: import {} not found\nExpected paths:\n{}",
                importer.display(),
                import.path().join("::"),
                expected
                    .iter()
                    .map(|path| format!(
                        "  {}",
                        std::path::absolute(path)
                            .unwrap_or_else(|_| path.clone())
                            .display()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportResolver {
    pub lib_dirs: Vec<PathBuf>,
}
impl ImportResolver {
    /// Explicit source roots only; installed packages are resolved by PathResolver.
    pub fn source_roots(&self) -> Vec<PathBuf> {
        self.lib_dirs.clone()
    }
    pub fn from_executable(_executable: &Path) -> Result<Self, String> {
        Ok(Self { lib_dirs: vec![] })
    }
    pub fn resolve(&self, importer: &Path, import: &ImportModule) -> Result<PathBuf, String> {
        let local = importer.parent().ok_or("Importing file has no directory")?;
        self.resolve_in_roots(
            import,
            std::iter::once(local).chain(self.lib_dirs.iter().map(PathBuf::as_path)),
            &[],
            false,
        )?
        .require(importer, import)
    }
    pub(super) fn resolve_in_roots<'a>(
        &self,
        import: &ImportModule,
        roots: impl IntoIterator<Item = &'a Path>,
        extra_objects: &'a [PathBuf],
        root_priority: bool,
    ) -> Result<ImportSearch, String> {
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
        let source_ext = SOURCE_SUFFIX.trim_start_matches('.');
        let mut candidates = Vec::new();
        for root in roots {
            for extension in [source_ext, "sym"] {
                let candidate = root.join(&relative).with_extension(extension);
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
        for root in extra_objects {
            let candidate = root.join(&relative).with_extension("sym");
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        if !root_priority {
            candidates.sort_by_key(|path| path.extension().is_some_and(|ext| ext == "sym"));
        }
        for candidate in &candidates {
            if candidate.is_file() {
                if candidate.extension().is_some_and(|ext| ext == "sym") {
                    crate::artifacts::load(candidate)?;
                }
                return candidate
                    .canonicalize()
                    .map(ImportSearch::Found)
                    .map_err(|e| e.to_string());
            }
        }
        Ok(ImportSearch::Missing(candidates))
    }
}
