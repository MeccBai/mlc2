use crate::cli::Cli;
use mlc_builder::project::TargetKind;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

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
        let mut command = Command::new(&cli.archiver);
        command.arg("rcs").arg(&staged).args(objects);
        command
    } else {
        let mut command = Command::new(&cli.linker);
        command.arg("-fuse-ld=lld");
        // The development UCRT driver accepts scalar/pointer Win64 objects emitted
        // for MSVC. Other targets are forwarded explicitly to the driver.
        if triplet != "x86_64-pc-windows-msvc" {
            command.arg(format!("--target={triplet}"));
        }
        if kind == TargetKind::Shared {
            command.arg("-shared");
        }
        command
            .args(objects)
            .arg("-o")
            .arg(&staged)
            .args(&cli.link_args);
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
