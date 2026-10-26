//! Target-specific import and artifact layout.
use crate::{config::GlobalConfig, plan::ImportResolver, project::ProjectFile};
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
mod tests;

/// Default target follows the platform this compiler executable was built for.
pub const HOST_TRIPLET: &str = env!("MLC_HOST_TRIPLET");

#[derive(Debug, Clone)]
pub struct PathResolver {
    pub mode: crate::project::BuildMode,
    pub std_type: Option<crate::project::StdType>,
    pub build_root: PathBuf,
    pub imports: ImportResolver,
    pub triplet: String,
    pub output: PathBuf,
    pub lib_cache: Option<PathBuf>,
    pub lib_fallback: PathBuf,
}

impl PathResolver {
    /// Target name selects a Project.Targets entry; no selection uses Project.Triplet.
    pub fn load(
        executable: &Path,
        project_root: &Path,
        target: Option<&str>,
    ) -> Result<Self, String> {
        Self::new(
            executable,
            project_root,
            target,
            &GlobalConfig::load_default()?,
        )
    }

    pub fn new(
        executable: &Path,
        project_root: &Path,
        target: Option<&str>,
        global: &GlobalConfig,
    ) -> Result<Self, String> {
        let project = ProjectFile::load(project_root)?;
        Self::with_project(executable, project_root, target, global, project)
    }

    /// Resolve a standalone source without reading any project configuration.
    pub fn standalone(
        executable: &Path,
        root: &Path,
        global: &GlobalConfig,
    ) -> Result<Self, String> {
        Self::with_project(executable, root, None, global, None)
    }

    fn with_project(
        executable: &Path,
        project_root: &Path,
        target: Option<&str>,
        global: &GlobalConfig,
        project: Option<ProjectFile>,
    ) -> Result<Self, String> {
        let selected = match (project.as_ref(), target) {
            (Some(project), Some(name)) => Some(
                project
                    .project
                    .targets
                    .iter()
                    .find(|t| t.name == name)
                    .ok_or_else(|| format!("Unknown project target: {name}"))?,
            ),
            (None, Some(_)) => return Err("Named target requires Project.toml".into()),
            _ => None,
        };
        let triplet = selected
            .and_then(|t| t.triplet.as_deref())
            .or_else(|| project.as_ref().and_then(|p| p.project.triplet.as_deref()))
            .unwrap_or(HOST_TRIPLET);
        validate_triplet(triplet)?;
        let output = project_root.join(
            project
                .as_ref()
                .map(|p| p.project.output_dir.as_path())
                .unwrap_or(Path::new("build")),
        );
        let installation = executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or("Executable has no installation root")?;
        let dirs = selected
            .and_then(|t| t.lib_dirs.as_ref())
            .or_else(|| project.as_ref().and_then(|p| p.project.lib_dirs.as_ref()));
        let lib_dirs = if let Some(dirs) = dirs {
            dirs.iter()
                .map(|dir| project_root.join(dir).join(triplet))
                .collect()
        } else if let Some(dirs) = &global.lib_dirs {
            dirs.iter().map(|dir| dir.join(triplet)).collect()
        } else {
            vec![installation.join(crate::manifest::LIB_DIR).join(triplet)]
        };
        let cache = selected.and_then(|t| t.lib_cache_dir.as_ref()).or_else(|| {
            project
                .as_ref()
                .and_then(|p| p.project.lib_cache_dir.as_ref())
        });
        let lib_cache = cache
            .map(|dir| project_root.join(dir))
            .or_else(|| global.lib_cache_dir.clone())
            .map(|dir| dir.join("debug").join(triplet));
        let mode = selected
            .and_then(|target| target.mode)
            .or_else(|| project.as_ref().and_then(|project| project.project.mode))
            .unwrap_or_default();
        let mut paths = Self {
            mode,
            std_type: selected.and_then(|target| target.std_type).or_else(|| {
                project
                    .as_ref()
                    .and_then(|project| project.project.std_type)
            }),
            build_root: output.clone(),
            imports: ImportResolver { lib_dirs },
            triplet: triplet.into(),
            lib_cache,
            lib_fallback: output.join("objects/lib").join(triplet),
            output: output.join("objects").join(triplet),
        };
        paths.configure_output(output, mode);
        Ok(paths)
    }

    pub fn std_mode(&self) -> crate::project::BuildMode {
        self.std_type
            .unwrap_or_else(|| crate::project::StdType::for_mode(self.mode))
            .mode()
    }

    pub fn configure_output(&mut self, root: PathBuf, mode: crate::project::BuildMode) {
        self.mode = mode;
        self.build_root = root;
        self.output = self
            .build_root
            .join("objects")
            .join(mode.directory())
            .join(&self.triplet);
        self.lib_fallback = self.output.join("lib");
        if let Some(cache) = self
            .lib_cache
            .as_ref()
            .and_then(|path| path.parent())
            .and_then(|path| path.parent())
        {
            self.lib_cache = Some(cache.join(self.std_mode().directory()).join(&self.triplet));
        }
    }

    pub fn is_std_module(module: &str) -> bool {
        module == "std" || module.starts_with("std::")
    }

    pub(crate) fn cached_declaration_roots(&self, module: &str) -> Vec<PathBuf> {
        let Some(cache) = self
            .lib_cache
            .as_ref()
            .and_then(|path| path.parent())
            .and_then(|path| path.parent())
        else {
            return vec![];
        };
        let mode = if Self::is_std_module(module) {
            self.std_mode()
        } else {
            self.mode
        };
        vec![
            cache.join(mode.directory()).join(&self.triplet),
            cache.join(&self.triplet),
            cache.join("universal"),
        ]
    }

    pub fn artifact_stems(&self, file: &Path, module: &str) -> Result<Vec<PathBuf>, String> {
        let relative = module_path(module)?;
        let library = self
            .imports
            .source_roots()
            .iter()
            .find(|root| root.canonicalize().is_ok_and(|root| file.starts_with(root)))
            .cloned();
        let mode = if Self::is_std_module(module) {
            self.std_mode()
        } else {
            self.mode
        };
        let fallback = self
            .build_root
            .join("objects")
            .join(mode.directory())
            .join(&self.triplet)
            .join("lib");
        Ok(if let Some(root) = library {
            vec![
                self.lib_cache
                    .as_ref()
                    .map(|dir| {
                        dir.parent()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join(mode.directory())
                            .join(&self.triplet)
                            .join(&relative)
                    })
                    .unwrap_or_else(|| {
                        let mut objects = ImportResolver::object_root(&root);
                        objects.pop();
                        objects
                            .join(mode.directory())
                            .join(&self.triplet)
                            .join(&relative)
                    }),
                fallback.join(relative),
            ]
        } else {
            vec![self.output.join(relative)]
        })
    }
}

pub fn validate_triplet(triplet: &str) -> Result<(), String> {
    if triplet.is_empty()
        || !triplet
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        || triplet == "."
        || triplet == ".."
    {
        return Err(format!("Invalid target triplet: {triplet}"));
    }
    Ok(())
}

fn module_path(module: &str) -> Result<PathBuf, String> {
    let mut path = PathBuf::new();
    for segment in module.split("::") {
        if segment.is_empty()
            || segment.contains(['/', '\\', ':', '.'])
            || !matches!(
                Path::new(segment).components().next(),
                Some(Component::Normal(_))
            )
        {
            return Err(format!("Invalid module name: {module}"));
        }
        path.push(segment);
    }
    Ok(path)
}
