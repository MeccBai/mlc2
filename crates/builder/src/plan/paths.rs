use mlc_manifest::SOURCE_SUFFIX;
use mlc_syntax::ImportModule;
use std::path::{Component, Path, PathBuf};
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
        self.resolve_with_modes(importer, import, None, &[])
    }
    pub(super) fn resolve_with_modes(
        &self,
        importer: &Path,
        import: &ImportModule,
        _modes: Option<(crate::project::BuildMode, crate::project::BuildMode)>,
        extra_objects: &[PathBuf],
    ) -> Result<PathBuf, String> {
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
        let mut expected = vec![];
        for extension in [SOURCE_SUFFIX.trim_start_matches('.'), "sym"] {
            for root in std::iter::once(local)
                .chain(self.lib_dirs.iter().map(PathBuf::as_path))
                .chain(
                    extra_objects
                        .iter()
                        .map(PathBuf::as_path)
                        .filter(|_| extension == "sym"),
                )
            {
                let candidate = root.join(&relative).with_extension(extension);
                if expected.contains(&candidate) {
                    continue;
                }
                expected.push(candidate.clone());
                if candidate.is_file() {
                    if extension == "sym" {
                        crate::artifacts::load(&candidate)?;
                    }
                    return candidate.canonicalize().map_err(|e| e.to_string());
                }
            }
        }
        Err(format!(
            "{}: import {} not found\nExpected paths:\n{}",
            importer.display(),
            import.path().join("::"),
            expected
                .iter()
                .map(|p| format!(
                    "  {}",
                    std::path::absolute(p)
                        .unwrap_or_else(|_| p.clone())
                        .display()
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}
