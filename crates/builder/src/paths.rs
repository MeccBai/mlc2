//! Target-specific import and artifact layout.
use crate::{config::GlobalConfig, plan::ImportResolver, project::ProjectFile};
use std::path::{Component, Path, PathBuf};
#[cfg(test)]
mod tests;

/// Default target follows the platform this compiler executable was built for.
pub const HOST_TRIPLET: &str = env!("MLC_HOST_TRIPLET");

#[derive(Debug, Clone)]
pub struct PathResolver {
    pub package_root: PathBuf,
    pub requirements: Vec<crate::packages::Requirement>,
    pub packages: Vec<crate::packages::Resolved>,
    pub toolchain_packages: Vec<crate::packages::Resolved>,
    pub package_identity: Option<(PathBuf, String)>,
    pub ctfe_max_time: std::time::Duration,
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
    ) -> Result<Self, mlc_core::diagnostic::Report> {
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
    ) -> Result<Self, mlc_core::diagnostic::Report> {
        let project = ProjectFile::load(project_root)?;
        Self::with_project(executable, project_root, target, global, project).map_err(Into::into)
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
            vec![]
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
            .and_then(|target| target.profile.or(target.mode))
            .or_else(|| project.as_ref().and_then(|project| project.project.mode))
            .unwrap_or_default();
        let mut paths = Self {
            package_root: crate::packages::installation(executable),
            requirements: selected.map(|t| t.requires.clone()).unwrap_or_default(),
            packages: vec![],
            toolchain_packages: vec![],
            package_identity: None,
            ctfe_max_time: project
                .as_ref()
                .map(|p| p.ctfe.max_time)
                .unwrap_or_else(|| crate::project::CtfeConfig::default().max_time),
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
        vec![cache.join(mode.directory()).join(&self.triplet)]
    }

    pub fn artifact_stems(&self, file: &Path, module: &str) -> Result<Vec<PathBuf>, String> {
        let relative = module_path(module)?;
        if let Some(package) = self
            .toolchain_packages
            .iter()
            .find(|p| p.root.canonicalize().is_ok_and(|r| file.starts_with(r)))
        {
            return Ok(vec![
                package
                    .root
                    .join(".compiled/objects")
                    .join(self.std_mode().directory())
                    .join(&self.triplet)
                    .join(&relative),
                self.output.join("packages").join(&relative),
            ]);
        }

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
        Ok(if library.is_some() {
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
                        self.package_root
                            .join("cache/loose")
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

impl PathResolver {
    pub fn resolve_packages(&mut self) -> Result<(), String> {
        let registry = crate::packages::Registry::load(&self.package_root)?;
        self.packages = registry.resolve(
            &self.package_root,
            &self.requirements,
            self.mode,
            &self.triplet,
        )?;
        self.toolchain_packages.clear();
        for name in ["c_std", "std"] {
            if self.packages.iter().any(|p| p.requirement.name == name) {
                continue;
            }
            if registry
                .packages
                .iter()
                .any(|p| p.name == name && p.version == env!("CARGO_PKG_VERSION"))
            {
                let requirement = crate::packages::Requirement {
                    name: name.into(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    kind: crate::packages::PackageType::Source,
                    link_type: None,
                };
                for package in registry.resolve(
                    &self.package_root,
                    &[requirement],
                    self.std_mode(),
                    &self.triplet,
                )? {
                    if !self
                        .toolchain_packages
                        .iter()
                        .any(|p| p.requirement.name == package.requirement.name)
                    {
                        self.toolchain_packages.push(package);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn package_module_name(&self, file: &Path) -> Option<String> {
        if let Some((entry, name)) = &self.package_identity {
            if entry.canonicalize().ok().as_deref() == Some(file) {
                return Some(name.clone());
            }
            if let Some(relative) = entry
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .and_then(|p| file.strip_prefix(p).ok())
            {
                return Some(format!(
                    "{name}::{}",
                    relative
                        .with_extension("")
                        .components()
                        .map(|p| p.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("::")
                ));
            }
        }
        for package in self.packages.iter().chain(&self.toolchain_packages) {
            if let Some(entry) = &package.entry {
                if entry.canonicalize().ok().as_deref() == Some(file) {
                    return Some(package.requirement.name.clone());
                }
                if let Some(relative) = entry
                    .parent()
                    .and_then(|p| p.canonicalize().ok())
                    .and_then(|p| file.strip_prefix(p).ok())
                {
                    return Some(format!(
                        "{}::{}",
                        package.requirement.name,
                        relative
                            .with_extension("")
                            .components()
                            .map(|p| p.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("::")
                    ));
                }
            }
        }
        None
    }
    pub fn package_import(
        &self,
        importer: &Path,
        import: &mlc_syntax::ImportModule,
    ) -> Result<Option<PathBuf>, String> {
        let segments = import.path();
        let Some(first) = segments.first() else {
            return Ok(None);
        };
        let contains = |root: &Path| root.canonicalize().is_ok_and(|r| importer.starts_with(r));
        let owner = self
            .packages
            .iter()
            .chain(&self.toolchain_packages)
            .find(|p| contains(&p.root) || contains(&p.symbols));
        let package = self
            .packages
            .iter()
            .chain(&self.toolchain_packages)
            .find(|p| p.requirement.name == *first);
        if let Some(package) = package {
            let permitted = self
                .toolchain_packages
                .iter()
                .any(|p| p.requirement.name == *first)
                || owner
                    .map(|owner| {
                        owner.requirement.name == *first
                            || owner.package.requires.iter().any(|r| r.name == *first)
                    })
                    .unwrap_or_else(|| self.requirements.iter().any(|r| r.name == *first));
            if !permitted {
                return Err(format!(
                    "Package {first} is not a direct dependency of this module"
                ));
            }
            let relative = segments.iter().collect::<PathBuf>();
            let candidate = if package.library.is_some() {
                package.symbols.join(relative).with_extension("sym")
            } else {
                let root = package
                    .entry
                    .as_ref()
                    .and_then(|p| p.parent())
                    .unwrap_or(&package.root);
                if segments.len() == 1 {
                    package.entry.clone().ok_or("Declaration-only packages have no root module; import a declaration submodule")?
                } else {
                    root.join(segments[1..].iter().collect::<PathBuf>())
                        .with_extension("m2")
                }
            };
            let candidate = if candidate.is_file() {
                candidate
            } else if package.library.is_none() {
                let root = package
                    .entry
                    .as_ref()
                    .and_then(|p| p.parent())
                    .unwrap_or(&package.root);
                let relative = if segments.len() == 1 {
                    PathBuf::from(first)
                } else {
                    segments[1..].iter().collect()
                };
                let declaration = [
                    package.root.join("targets").join(&self.triplet),
                    root.to_owned(),
                ]
                .into_iter()
                .find_map(|root| {
                    ["h", "toml"]
                        .into_iter()
                        .map(|extension| root.join(&relative).with_extension(extension))
                        .find(|p| p.is_file())
                });
                if declaration.is_some() {
                    let symbol = package
                        .root
                        .join(".compiled/declarations")
                        .join(&self.triplet)
                        .join(segments.iter().collect::<PathBuf>())
                        .with_extension("sym");
                    if !symbol.is_file() {
                        return Err(format!(
                            "Package declaration is not prepared. Run mlc setup.\nExpected path: {}",
                            symbol.display()
                        ));
                    }
                    symbol
                } else {
                    candidate
                }
            } else {
                candidate
            };
            if !candidate.is_file() {
                return Err(format!(
                    "Package module not found\nExpected path: {}",
                    candidate.display()
                ));
            }
            return candidate
                .canonicalize()
                .map(Some)
                .map_err(|e| e.to_string());
        }
        if let Some(owner) = owner.filter(|p| p.library.is_some()) {
            let candidate = owner
                .symbols
                .join(&owner.requirement.name)
                .join(segments.iter().collect::<PathBuf>())
                .with_extension("sym");
            if candidate.is_file() {
                return candidate
                    .canonicalize()
                    .map(Some)
                    .map_err(|e| e.to_string());
            }
        }
        if let Some((entry, name)) = &self.package_identity {
            let relative: PathBuf = if first == name {
                segments[1..].iter().collect()
            } else {
                segments.iter().collect()
            };
            let root = entry.parent().unwrap();
            if ["h", "toml"].into_iter().any(|ext| {
                root.join(&relative).with_extension(ext).is_file()
                    || root
                        .join("targets")
                        .join(&self.triplet)
                        .join(&relative)
                        .with_extension(ext)
                        .is_file()
            }) {
                let candidate = root
                    .join(".compiled/declarations")
                    .join(&self.triplet)
                    .join(name)
                    .join(&relative)
                    .with_extension("sym");
                if candidate.is_file() {
                    return candidate
                        .canonicalize()
                        .map(Some)
                        .map_err(|e| e.to_string());
                }
                return Err(format!(
                    "Package declaration is not prepared. Run mlc setup.\nExpected path: {}",
                    candidate.display()
                ));
            }
            if first == name {
                let candidate = if segments.len() == 1 {
                    entry.clone()
                } else {
                    entry
                        .parent()
                        .unwrap()
                        .join(segments[1..].iter().collect::<PathBuf>())
                        .with_extension("m2")
                };
                if candidate.is_file() {
                    return candidate
                        .canonicalize()
                        .map(Some)
                        .map_err(|e| e.to_string());
                }
            }
        }
        Ok(None)
    }
}
