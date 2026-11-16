//! Build installed sources as separate libraries; publish installations only after success.
use super::*;
use mlc_builder::{
    artifacts::{self, ObjectMode},
    packages::{self, Binary, LinkType, Package, PackageType, Registry, Source, Variants},
    plan::TargetInput,
};
use std::{fs, io::Write, path::PathBuf};
fn contains(root: &Path, path: &Path) -> bool {
    root.canonicalize().is_ok_and(|root| path.starts_with(root))
}
fn library_cli(
    cli: &Cli,
    paths: &PathResolver,
    entry: &Path,
    name: &str,
    output: PathBuf,
    requires: Vec<mlc_builder::packages::Requirement>,
) -> Cli {
    let mut child = cli.clone();
    child.command = Commands::Build { target: None };
    child.package_requires = Some(requires);
    child.package_ctfe_max_time = Some(paths.ctfe_max_time);
    child.package_std_type = paths.std_type;
    child.package_identity = Some((entry.to_owned(), name.into()));
    child.output = Some(output);
    child.triplet = Some(paths.triplet.clone());
    child.default_mode = false;
    child.debug = packages::profile(paths.mode) == mlc_builder::project::BuildMode::Debug;
    child.release = !child.debug;
    child.rel_with_info = false;
    child.optimize = false;
    child.no_optimize = false;
    child.kind = None;
    child.lib_dirs = paths
        .imports
        .lib_dirs
        .iter()
        .filter_map(|p| p.parent().map(Path::to_owned))
        .collect();
    child
}
fn source_hash(root: &Path) -> Result<String, String> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<(PathBuf, String)>) -> Result<(), String> {
        let mut entries = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!(
                    "Package sources cannot contain symlinks: {}",
                    path.display()
                ));
            }
            if kind.is_dir() {
                if !entry.file_name().to_string_lossy().starts_with('.')
                    && entry.file_name() != "build"
                    && entry.file_name() != "target"
                {
                    visit(root, &path, files)?;
                }
            } else if path
                .extension()
                .is_some_and(|e| e == "m2" || e == "h" || e == "toml")
            {
                files.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    artifacts::content_hash(&fs::read(&path).map_err(|e| e.to_string())?),
                ));
            }
        }
        Ok(())
    }
    let mut files = vec![];
    visit(root, root, &mut files)?;
    artifacts::semantic_hash(&files)
}
pub(super) fn prepare(
    cli: &Cli,
    global: &GlobalConfig,
    paths: &mut PathResolver,
    progress: &Progress,
) -> Result<(), Report> {
    for i in 0..paths.packages.len() {
        let package = paths.packages[i].clone();
        if package.requirement.kind != PackageType::Source {
            continue;
        }
        if package
            .package
            .source
            .as_ref()
            .is_some_and(|s| s.declaration_only)
        {
            packages::prepare_declarations(&package, &paths.triplet)?;
            continue;
        }
        let entry = package.entry.as_ref().unwrap();
        let dependencies = paths.packages[..i]
            .iter()
            .filter_map(|p| p.library.as_ref())
            .map(|p| {
                fs::read(p)
                    .map(|b| artifacts::content_hash(&b))
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let key = artifacts::semantic_hash(&(
            "package-v1",
            &package.requirement,
            &paths.triplet,
            package.profile.directory(),
            source_hash(&package.root)?,
            dependencies,
            mlc_manifest::BUILD_NUMBER,
        ))?;
        let output = paths.package_root.join("cache/artifacts").join(key);
        let child = library_cli(
            cli,
            paths,
            entry,
            &package.requirement.name,
            output.clone(),
            package.package.requires.clone(),
        );
        let kind = if package.requirement.linkage() == LinkType::Shared {
            TargetKind::Shared
        } else {
            TargetKind::Static
        };
        let library = super::build(
            &child,
            global,
            entry.parent().unwrap(),
            entry,
            None,
            kind,
            progress,
            false,
        )?
        .ok_or("Source package produced no library")?;
        paths.packages[i].library = Some(
            if kind == TargetKind::Shared && paths.triplet.contains("windows") {
                library.with_extension("lib")
            } else {
                library.clone()
            },
        );
        paths.packages[i].runtime = (kind == TargetKind::Shared).then_some(library);
        paths.packages[i].symbols = output
            .join("objects")
            .join(package.profile.directory())
            .join(&paths.triplet);
    }
    Ok(())
}
pub(super) fn link_inputs(paths: &PathResolver, objects: &mut Vec<PathBuf>, kind: TargetKind) {
    objects.retain(|p| {
        !paths
            .packages
            .iter()
            .any(|package| contains(&package.symbols, p))
    });
    if kind == TargetKind::Static {
        return;
    }
    for package in paths.packages.iter().rev() {
        if let Some(library) = &package.library {
            if !objects.contains(library) {
                objects.push(library.clone());
            }
        }
    }
}
pub(super) fn copy_runtimes(paths: &PathResolver, output: &Path) -> Result<(), String> {
    let mut names = std::collections::HashMap::new();
    for package in &paths.packages {
        if let Some(runtime) = &package.runtime {
            let name = runtime.file_name().ok_or("Runtime has no filename")?;
            let bytes = fs::read(runtime).map_err(|e| e.to_string())?;
            let hash = artifacts::content_hash(&bytes);
            if names
                .insert(name.to_owned(), hash.clone())
                .is_some_and(|old| old != hash)
            {
                return Err(format!(
                    "Conflicting package runtime filename: {}",
                    name.to_string_lossy()
                ));
            }
            let target = output.join(name);
            if target != *runtime && fs::read(&target).ok().as_deref() != Some(bytes.as_slice()) {
                let mut staged =
                    tempfile::NamedTempFile::new_in(output).map_err(|e| e.to_string())?;
                staged.write_all(&bytes).map_err(|e| e.to_string())?;
                staged.persist(target).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}
fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err("Package artifacts cannot contain symlinks".into());
        }
        if kind.is_dir() {
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn rewrite_artifacts(
    directory: &Path,
    old_source: &Path,
    new_source: &Path,
    binary: bool,
) -> Result<(), String> {
    let normalized = old_source
        .canonicalize()
        .unwrap_or_else(|_| old_source.to_owned());
    let old_source = normalized.as_path();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            rewrite_artifacts(&path, old_source, new_source, binary)?;
        } else if path.extension().is_some_and(|e| e == "sym") {
            let mut manifest: artifacts::Manifest =
                artifacts::from_binary(&fs::read(&path).map_err(|e| e.to_string())?)?;
            if let Ok(relative) = Path::new(&manifest.config.source_file).strip_prefix(old_source) {
                manifest.config.source_file = new_source.join(relative).to_string_lossy().into();
            }
            if binary {
                manifest.config.object = ObjectMode::Only;
            }
            if let Some(name) = &manifest.config.generic {
                let generic = path.parent().unwrap().join(name);
                let mut bundle: artifacts::GenericBundle =
                    artifacts::from_binary(&fs::read(&generic).map_err(|e| e.to_string())?)?;
                bundle.source_file = manifest.config.source_file.clone();
                let bytes = artifacts::to_binary(&bundle)?;
                manifest.config.generic_file_hash = Some(artifacts::content_hash(&bytes));
                fs::write(&generic, bytes).map_err(|e| e.to_string())?;
            }
            fs::write(&path, artifacts::to_binary(&manifest)?).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
struct CatalogueLock(PathBuf);
impl Drop for CatalogueLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
pub(super) fn install(
    cli: &Cli,
    root: &Path,
    target: &str,
    representation: crate::cli::InstallType,
    progress: &Progress,
) -> Result<i32, Report> {
    let project = ProjectFile::load(root)?.ok_or("mlc install requires Project.toml")?;
    let target = project
        .project
        .targets
        .iter()
        .find(|t| t.name == target)
        .ok_or("No matching library target")?;
    if target.kind == TargetKind::Bin {
        return Err("mlc install requires a lib, static or shared target".into());
    }
    packages::component(&target.name)?;
    packages::version(&project.project.version)?;
    let global = match &cli.config {
        Some(path) => GlobalConfig::load(&root.join(path))?,
        None => GlobalConfig::load_default()?,
    };
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut paths = PathResolver::new(&executable, root, Some(&target.name), &global)?;
    if let Some(triplet) = &cli.triplet {
        paths.triplet = triplet.clone();
    }
    paths.configure_output(
        paths.build_root.clone(),
        packages::profile(cli.build_mode(paths.mode)),
    );
    if !cli.lib_dirs.is_empty() {
        paths.imports.lib_dirs = cli
            .lib_dirs
            .iter()
            .map(|p| root.join(p).join(&paths.triplet))
            .collect();
    }
    paths.resolve_packages()?;
    let representation = match representation {
        crate::cli::InstallType::Source => PackageType::Source,
        crate::cli::InstallType::Static => PackageType::Static,
        crate::cli::InstallType::Shared => PackageType::Shared,
    };
    let parent = paths
        .package_root
        .join("packages")
        .join(&target.name)
        .join(&project.project.version);
    let parent = if representation == PackageType::Source {
        parent
    } else {
        parent.join(paths.mode.directory())
    };
    let destination = parent.join(representation.directory());
    if destination.exists() {
        return Err(format!("Package representation already installed: {}. Increase Project.Version to publish changed sources.",destination.display()).into());
    }
    fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".install-")
        .tempdir_in(&parent)
        .map_err(|e| e.to_string())?;
    let entry = root.join(&target.entry).canonicalize().map_err(|e| {
        format!(
            "Cannot open library entry {}: {e}",
            root.join(&target.entry).display()
        )
    })?;
    let project_root = root.canonicalize().map_err(|e| e.to_string())?;
    paths.package_identity = Some((entry.clone(), target.name.clone()));
    let plan = BuildPlan::discover_with_paths(&entry, &paths)?;
    for module in &plan.targets {
        if matches!(module.input, TargetInput::Source { .. }) && module.artifact_stems.len() <= 1 {
            if let Ok(relative) = module.file.strip_prefix(&project_root) {
                let to = stage.path().join(relative);
                fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
                fs::copy(&module.file, to).map_err(|e| e.to_string())?;
            }
        }
    }
    let relative = entry
        .strip_prefix(&project_root)
        .map_err(|_| "Library entry must be inside the project directory")?;
    let installed_entry = stage.path().join(relative);
    if !installed_entry.is_file() {
        return Err("Library entry was not included in the package".into());
    }
    let output = stage.path().join(".compiled");
    let child = library_cli(
        cli,
        &paths,
        &installed_entry,
        &target.name,
        output.clone(),
        target.requires.clone(),
    );
    let kind = if representation == PackageType::Shared {
        TargetKind::Shared
    } else {
        TargetKind::Static
    };
    let artifact = super::build(
        &child,
        &global,
        stage.path(),
        &installed_entry,
        None,
        kind,
        progress,
        false,
    )?
    .ok_or("Installation build produced no library")?;
    let symbols = output
        .join("objects")
        .join(paths.mode.directory())
        .join(&paths.triplet);
    let publish = if representation == PackageType::Source {
        rewrite_artifacts(&symbols, stage.path(), &destination, false)?;
        stage.path().to_owned()
    } else {
        let publish = stage.path().join(".publish");
        fs::create_dir(&publish).map_err(|e| e.to_string())?;
        let modules = publish.join("modules");
        fs::create_dir(&modules).map_err(|e| e.to_string())?;
        let entry_symbols = symbols.join(&target.name).with_extension("sym");
        for suffix in ["sym", "obj", "o", "mg", "pdb"] {
            let file = entry_symbols.with_extension(suffix);
            if file.is_file() {
                fs::copy(&file, modules.join(file.file_name().unwrap()))
                    .map_err(|e| e.to_string())?;
            }
        }
        let nested = symbols.join(&target.name);
        if nested.is_dir() {
            copy_tree(&nested, &modules.join(&target.name))?;
        }
        rewrite_artifacts(&modules, stage.path(), &destination, true)?;
        fs::copy(&artifact, publish.join(artifact.file_name().unwrap()))
            .map_err(|e| e.to_string())?;
        if kind == TargetKind::Shared && paths.triplet.contains("windows") {
            let import = artifact.with_extension("lib");
            fs::copy(&import, publish.join(import.file_name().unwrap()))
                .map_err(|e| e.to_string())?;
        }
        publish
    };
    fs::create_dir_all(&paths.package_root).map_err(|e| e.to_string())?;
    let lock_path = paths.package_root.join("packages/packages.toml.lock");
    let _lock_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| format!("Cannot lock {}: {e}", lock_path.display()))?;
    drop(_lock_file);
    let _lock = CatalogueLock(lock_path);
    let mut registry = Registry::load(&paths.package_root)?;
    let index = if let Some(index) = registry
        .packages
        .iter()
        .position(|p| p.name == target.name && p.version == project.project.version)
    {
        if registry.packages[index].requires != target.requires {
            return Err("Package representations must declare the same dependencies".into());
        }
        index
    } else {
        registry.packages.push(Package {
            name: target.name.clone(),
            version: project.project.version.clone(),
            requires: target.requires.clone(),
            source: None,
            debug: None,
            release: None,
        });
        registry.packages.len() - 1
    };
    if representation == PackageType::Source {
        registry.packages[index].source = Some(Source {
            entry: Some(relative.to_owned()),
            triplet: None,
            declaration_only: false,
        });
    } else {
        let profile = if paths.mode == mlc_builder::project::BuildMode::Release {
            &mut registry.packages[index].release
        } else {
            &mut registry.packages[index].debug
        };
        let variants = profile.get_or_insert_with(Variants::default);
        match representation {
            PackageType::Source => unreachable!(),
            PackageType::Static => {
                variants.static_lib = Some(Binary {
                    library: artifact.file_name().unwrap().into(),
                    symbols: "modules".into(),
                    runtime: None,
                    triplet: paths.triplet.clone(),
                })
            }
            PackageType::Shared => {
                variants.shared = Some(Binary {
                    library: if paths.triplet.contains("windows") {
                        artifact.with_extension("lib").file_name().unwrap().into()
                    } else {
                        artifact.file_name().unwrap().into()
                    },
                    symbols: "modules".into(),
                    runtime: Some(artifact.file_name().unwrap().into()),
                    triplet: paths.triplet.clone(),
                })
            }
        }
    }
    registry.validate()?;
    fs::rename(&publish, &destination).map_err(|e| e.to_string())?;
    if let Err(error) = registry.save(&paths.package_root) {
        let _ = fs::rename(&destination, &publish);
        return Err(error.into());
    }
    crate::output::artifact("package", &destination);
    crate::output::info(format!(
        "Installed {} {} ({}/{:?}) at {}",
        target.name,
        project.project.version,
        paths.mode.directory(),
        representation,
        destination.display()
    ));
    Ok(0)
}
