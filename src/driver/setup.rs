//! Bootstrap all library inputs; declaration conversion precedes source compilation.
use super::{Progress, build};
use crate::cli::{Cli, Commands};
use mlc_builder::{
    artifacts,
    config::GlobalConfig,
    paths::{HOST_TRIPLET, validate_triplet},
    project::{BuildMode, TargetKind},
};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

fn scan(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|error| {
        format!(
            "Cannot read library directory: {error}\nExpected path: {}",
            root.display()
        )
    })? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            scan(&entry.path(), files)?;
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "h" || ext == "toml" || ext == "m2")
        {
            files.push(entry.path());
        }
    }
    Ok(())
}

struct Input {
    root: PathBuf,
    path: PathBuf,
    variant: String,
    module: String,
    relative: PathBuf,
}

pub(super) fn run(cli: &Cli, cwd: &Path, progress: &Progress) -> Result<i32, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let installation = executable
        .parent()
        .ok_or("Compiler has no installation directory")?;
    let roots = if cli.lib_dirs.is_empty() {
        vec![installation.join("lib")]
    } else {
        cli.lib_dirs.iter().map(|root| cwd.join(root)).collect()
    };
    let mut inputs = Vec::new();
    let mut seen = HashSet::new();
    let mut library_roots = Vec::new();
    let mut targets = BTreeSet::new();
    for root in roots {
        let root = root.canonicalize().map_err(|error| {
            format!(
                "Library directory not found: {error}\nExpected path: {}",
                root.display()
            )
        })?;
        library_roots.push(root.clone());
        let mut files = Vec::new();
        scan(&root, &mut files)?;
        files.sort();
        for path in files {
            if !seen.insert(path.clone()) {
                continue;
            }
            let relative = path.strip_prefix(&root).unwrap().to_owned();
            let mut parts = relative.components();
            let variant = parts
                .next()
                .unwrap()
                .as_os_str()
                .to_str()
                .ok_or("Non-UTF8 library target")?
                .to_owned();
            if parts.as_path().as_os_str().is_empty() {
                return Err(format!(
                    "Library inputs require lib/<triplet-or-universal>/<module> layout: {}",
                    path.display()
                ));
            }
            if variant != "universal" {
                validate_triplet(&variant)?;
                targets.insert(variant.clone());
            }
            let module = parts
                .as_path()
                .with_extension("")
                .iter()
                .map(|part| part.to_str().ok_or("Non-UTF8 module name"))
                .collect::<Result<Vec<_>, _>>()?
                .join("::");
            inputs.push(Input {
                root: root.clone(),
                path,
                variant,
                module,
                relative,
            });
        }
    }
    if let Some(target) = &cli.triplet {
        validate_triplet(target)?;
        targets = BTreeSet::from([target.clone()]);
    }
    if targets.is_empty() {
        targets.insert(HOST_TRIPLET.into());
    }
    if inputs.is_empty() {
        return Err(format!(
            "No library inputs found\nExpected paths:\n{}",
            library_roots
                .iter()
                .map(|root| format!(
                    "  {}/<triplet-or-universal>/**/*.m2, *.h, *.toml",
                    root.display()
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let output_override = cli.output.as_ref().map(|root| cwd.join(root));
    let destination = |input: &Input| {
        output_override
            .clone()
            .unwrap_or_else(|| input.root.parent().unwrap().to_owned())
    };
    let declarations: Vec<_> = inputs
        .iter()
        .filter(|input| {
            input
                .path
                .extension()
                .is_some_and(|ext| ext == "h" || ext == "toml")
                && (input.variant == "universal" || targets.contains(&input.variant))
        })
        .collect();
    let mut outputs = HashSet::new();
    progress.stage("Preparing library declarations", declarations.len());
    for input in &declarations {
        let output = destination(input)
            .join("objects")
            .join(&input.relative)
            .with_extension("sym");
        if !outputs.insert(output.clone()) {
            return Err(format!(
                "Duplicate library declaration output: {}",
                output.display()
            ));
        }
        if input.path.extension().is_some_and(|ext| ext == "h") {
            artifacts::convert_c_header(&input.path, &output, &input.module, &input.variant)?;
        } else {
            artifacts::convert_toml(&input.path, &output)?;
        }
        println!("Prepared {}", output.display());
        progress.advance();
    }
    let modes = if cli.default_mode || cli.debug || cli.release || cli.rel_with_info {
        vec![if cli.build_mode(BuildMode::Default).optimized() {
            BuildMode::Release
        } else {
            BuildMode::Debug
        }]
    } else {
        vec![BuildMode::Debug, BuildMode::Release]
    };
    let global = GlobalConfig {
        lib_dirs: Some(library_roots),
        lib_cache_dir: output_override.as_ref().map(|root| root.join("objects")),
    };
    let mut count = 0;
    for target in &targets {
        for mode in &modes {
            for input in inputs.iter().filter(|input| {
                input.path.extension().is_some_and(|ext| ext == "m2")
                    && (input.variant == "universal" || &input.variant == target)
            }) {
                let mut job = cli.clone();
                job.command = Commands::Setup;
                job.triplet = Some(target.clone());
                job.lib_dirs.clear();
                job.output = Some(destination(input));
                job.default_mode = false;
                job.debug = *mode == BuildMode::Debug;
                job.release = *mode == BuildMode::Release;
                job.rel_with_info = false;
                job.optimize = false;
                job.no_optimize = false;
                build(
                    &job,
                    &global,
                    cwd,
                    &input.path,
                    None,
                    TargetKind::Static,
                    progress,
                    true,
                )?;
                count += 1;
            }
        }
    }
    println!(
        "Library setup complete: {} declarations, {count} source builds",
        declarations.len()
    );
    Ok(0)
}
