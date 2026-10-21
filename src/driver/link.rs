use crate::cli::Cli;
use mlc_builder::project::TargetKind;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

mod targets;

pub fn archiver_path(cli: &Cli) -> Result<PathBuf, String> {
    if let Some(path) = &cli.archiver {
        return Ok(path.clone());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(exe
        .parent()
        .ok_or("Compiler has no parent")?
        .join(mlc_builder::manifest::TOOLS_DIR)
        .join(format!("llvm-ar{}", std::env::consts::EXE_SUFFIX)))
}

pub fn linker_path(cli: &Cli, triplet: &str) -> Result<PathBuf, String> {
    let root = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(cli.linker.clone().unwrap_or(
        root.parent()
            .ok_or("Compiler has no parent")?
            .join(mlc_builder::manifest::TOOLS_DIR)
            .join(format!(
                "{}{}",
                targets::profile(triplet)?.linker,
                std::env::consts::EXE_SUFFIX
            )),
    ))
}

pub fn runtime_identity(triplet: &str) -> Result<Vec<(PathBuf, String)>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let root = exe
        .parent()
        .ok_or("Compiler has no parent")?
        .join(mlc_builder::manifest::RUNTIMES_DIR)
        .join(triplet);
    let mut files = targets::profile(triplet)?.runtime_files(&root)?;
    files.sort();
    files
        .into_iter()
        .filter(|path| path.is_file())
        .map(|path| {
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            Ok((path, mlc_builder::artifacts::content_hash(&bytes)))
        })
        .collect()
}

pub fn output_path(root: &Path, stem: &str, kind: TargetKind, triplet: &str) -> PathBuf {
    let windows = triplet.contains("windows");
    root.join(match kind {
        TargetKind::Bin => format!("{stem}{}", if windows { ".exe" } else { "" }),
        TargetKind::Static => {
            if windows {
                format!("{stem}.lib")
            } else {
                format!("lib{stem}.a")
            }
        }
        TargetKind::Shared => {
            if windows {
                format!("{stem}.dll")
            } else {
                format!("lib{stem}.so")
            }
        }
    })
}

pub fn link(
    cli: &Cli,
    objects: &[PathBuf],
    output: &Path,
    kind: TargetKind,
    triplet: &str,
) -> Result<(), String> {
    let parent = output.parent().ok_or("Output has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let staged = staging
        .path()
        .join(output.file_name().ok_or("Output has no filename")?);
    let mut command = if kind == TargetKind::Static {
        let mut command = Command::new(archiver_path(cli)?);
        command.arg("rcs").arg(&staged).args(objects);
        command
    } else {
        let lld = linker_path(cli, triplet)?;
        let mut command = Command::new(&lld);
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        if !lld.is_file() {
            return Err(format!("Distribution linker not found: {}", lld.display()));
        }
        let runtime = executable
            .parent()
            .ok_or("Compiler has no parent")?
            .join(mlc_builder::manifest::RUNTIMES_DIR)
            .join(triplet);
        command.args(targets::profile(triplet)?.arguments(&runtime, objects, &staged, kind)?);
        command.args(&cli.link_args);
        command
    };
    let result = command
        .output()
        .map_err(|e| format!("Cannot launch linker/archiver: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "Link failed ({}):\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    if !result.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&result.stderr));
    }
    let published = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    std::fs::copy(&staged, published.path()).map_err(|e| e.to_string())?;
    published.persist(output).map_err(|e| e.to_string())?;
    Ok(())
}
