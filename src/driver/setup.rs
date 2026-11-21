//! Prepare source-package declarations and objects in their own package directories.
use super::{Progress, build};
use crate::cli::{Cli, Commands};
use mlc_builder::{
    config::GlobalConfig,
    packages::{self, PackageType, Registry, Requirement},
    paths::{HOST_TRIPLET, validate_triplet},
    project::{BuildMode, TargetKind},
};
use mlc_core::diagnostic::Report;
use std::{collections::BTreeSet, fs, path::Path};
pub(super) fn run(cli: &Cli, _cwd: &Path, progress: &Progress) -> Result<i32, Report> {
    if !cli.lib_dirs.is_empty() || cli.output.is_some() {
        return Err(
            "setup uses the MLC package catalogue; set MLC to select the installation directory"
                .into(),
        );
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let root = packages::installation(&executable);
    let registry = Registry::load(&root)?;
    if registry.packages.is_empty() {
        return Err(format!(
            "No packages registered\nExpected path: {}\nSet MLC to the directory containing packages/packages.toml, or unset MLC to use automatic discovery.",
            root.join("packages/packages.toml").display()
        )
        .into());
    }
    let modes = if cli.default_mode || cli.debug || cli.release || cli.rel_with_info {
        vec![packages::profile(cli.build_mode(BuildMode::Default))]
    } else {
        vec![BuildMode::Debug, BuildMode::Release]
    };
    let mut count = 0;
    let mut declarations = 0;
    let mut jobs = Vec::new();
    for package in &registry.packages {
        for mode in &modes {
            let Some(source) = package.source.as_ref() else {
                continue;
            };
            let mut targets = BTreeSet::from([HOST_TRIPLET.to_owned()]);
            let directory = root
                .join("packages")
                .join(&package.name)
                .join(&package.version)
                .join("source/targets");
            if directory.is_dir() {
                for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                        targets.insert(entry.file_name().to_string_lossy().into_owned());
                    }
                }
            }
            if let Some(triplet) = &cli.triplet {
                validate_triplet(triplet)?;
                targets = BTreeSet::from([triplet.clone()]);
            } else if let Some(triplet) = &source.triplet {
                targets = BTreeSet::from([triplet.clone()]);
            }
            let requirement = Requirement {
                name: package.name.clone(),
                version: package.version.clone(),
                kind: PackageType::Source,
                link_type: None,
            };
            for triplet in targets {
                let resolved = registry.resolve(&root, &[requirement.clone()], *mode, &triplet)?;
                let source = resolved
                    .into_iter()
                    .find(|p| p.requirement.name == package.name)
                    .unwrap();
                jobs.push((source, triplet, *mode));
            }
        }
    }
    let mut prepared = std::collections::HashSet::new();
    for (source, triplet, _) in &jobs {
        if prepared.insert((source.root.clone(), triplet.clone())) {
            declarations += packages::prepare_declarations(source, triplet)?;
        }
    }
    for (source, triplet, mode) in jobs {
        if source
            .package
            .source
            .as_ref()
            .is_some_and(|s| s.declaration_only)
        {
            continue;
        }
        for (_, input, _) in packages::source_inputs(&source, &triplet)?
            .into_iter()
            .filter(|(_, path, _)| path.extension().is_some_and(|e| e == "m2"))
        {
            let mut job = cli.clone();
            job.command = Commands::Setup;
            job.triplet = Some(triplet.clone());
            job.output = Some(source.root.join(".compiled/build"));
            job.package_identity = Some((
                source.entry.as_ref().unwrap().clone(),
                source.requirement.name.clone(),
            ));
            job.package_requires = Some(source.package.requires.clone());
            job.default_mode = false;
            job.debug = mode == BuildMode::Debug;
            job.release = mode == BuildMode::Release;
            job.rel_with_info = false;
            job.optimize = false;
            job.no_optimize = false;
            build(
                &job,
                &GlobalConfig::default(),
                &source.root,
                &input,
                None,
                TargetKind::Static,
                progress,
                true,
            )?;
            count += 1;
        }
    }
    crate::output::info(format!(
        "Package setup complete: {declarations} declarations, {count} source builds"
    ));
    Ok(0)
}
