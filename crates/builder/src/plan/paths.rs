use mlc_syntax::{ImportModule, manifest::SOURCE_SUFFIX};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ImportResolver {
    pub lib_dirs: Vec<PathBuf>,
}

impl ImportResolver {
    /// Source roots in priority order, followed by shared, target-independent sources.
    pub fn source_roots(&self) -> Vec<PathBuf> {
        let mut roots = self.lib_dirs.clone();
        for root in &self.lib_dirs {
            if let Some(parent) = root.parent() {
                let universal = parent.join(crate::manifest::UNIVERSAL_LIB_DIR);
                if !roots.contains(&universal) {
                    roots.push(universal);
                }
            }
        }
        roots
    }

    /// A configured lib/<triplet> has its artifacts in objects/<triplet>.
    pub fn object_root(root: &Path) -> PathBuf {
        let library = root.parent().unwrap_or(root);
        library
            .parent()
            .unwrap_or(library)
            .join(crate::manifest::OBJECTS_DIR)
            .join(root.file_name().unwrap_or_default())
    }

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
        self.resolve_with_modes(importer, import, None, &[])
    }

    pub(super) fn resolve_with_modes(
        &self,
        importer: &Path,
        import: &ImportModule,
        modes: Option<(crate::project::BuildMode, crate::project::BuildMode)>,
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
        let sources = self.source_roots();
        let mut expected = Vec::new();
        for root in std::iter::once(local).chain(sources.iter().map(PathBuf::as_path)) {
            let candidate = root
                .join(&relative)
                .with_extension(SOURCE_SUFFIX.trim_start_matches('.'));
            if !expected.contains(&candidate) {
                expected.push(candidate.clone());
            }
            if candidate.is_file() {
                return candidate.canonicalize().map_err(|e| e.to_string());
            }
        }
        // Explicit local artifacts win. Canonical declaration-only inputs then
        // precede old per-mode copies left by earlier installations.
        let local_symbol = local.join(&relative).with_extension("sym");
        if !expected.contains(&local_symbol) {
            expected.push(local_symbol.clone());
        }
        if local_symbol.is_file() {
            if local
                .file_name()
                .is_some_and(|name| name == crate::manifest::UNIVERSAL_LIB_DIR)
                && crate::artifacts::load(&local_symbol)?
                    .manifest
                    .config
                    .object
                    != crate::artifacts::ObjectMode::None
            {
                return Err(format!(
                    "Universal library must use Object = None: {}",
                    local_symbol.display()
                ));
            }
            return local_symbol
                .canonicalize()
                .map_err(|error| error.to_string());
        }
        let canonical_objects: Vec<_> = extra_objects
            .iter()
            .skip(1)
            .cloned()
            .chain(sources.iter().map(|root| Self::object_root(root)))
            .collect();
        for root in canonical_objects {
            let candidate = root.join(&relative).with_extension("sym");
            if !expected.contains(&candidate) {
                expected.push(candidate.clone());
            }
            if candidate.is_file() {
                let bytes = std::fs::read(&candidate)
                    .map_err(|error| format!("{}: {error}", candidate.display()))?;
                let manifest: crate::artifacts::Manifest = crate::artifacts::from_binary(&bytes)?;
                if manifest.config.object == crate::artifacts::ObjectMode::None {
                    crate::artifacts::load(&candidate)?;
                    return candidate.canonicalize().map_err(|error| error.to_string());
                }
            }
        }
        let mut objects = Vec::new();
        if let Some((mode, std_mode)) = modes {
            let mode = if import.path().first().is_some_and(|s| s == "std") {
                std_mode
            } else {
                mode
            };
            for root in &self.lib_dirs {
                let objects_root = Self::object_root(root);
                objects.push(
                    objects_root
                        .parent()
                        .unwrap()
                        .join(mode.directory())
                        .join(root.file_name().unwrap()),
                );
            }
        }
        objects.extend_from_slice(extra_objects);
        objects.extend(sources.iter().map(|root| Self::object_root(root)));
        for root in std::iter::once(local).chain(objects.iter().map(PathBuf::as_path)) {
            let candidate = root.join(&relative).with_extension("sym");
            if !expected.contains(&candidate) {
                expected.push(candidate.clone());
            }
            if candidate.is_file() {
                if root
                    .file_name()
                    .is_some_and(|name| name == crate::manifest::UNIVERSAL_LIB_DIR)
                    && crate::artifacts::load(&candidate)?.manifest.config.object
                        != crate::artifacts::ObjectMode::None
                {
                    return Err(format!(
                        "Universal library must use Object = None: {}",
                        candidate.display()
                    ));
                }
                return candidate.canonicalize().map_err(|e| e.to_string());
            }
        }
        Err(format!(
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
                .join("\n")
        ))
    }
}
