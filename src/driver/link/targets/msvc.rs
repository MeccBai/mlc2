//! Find import/startup libraries, never runtime DLLs from System32.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn newest(root: &Path, suffix: &Path, required: &str) -> Option<PathBuf> {
    let mut candidates: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    candidates.sort_by_key(|path| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0))
            .collect::<Vec<_>>()
    });
    candidates
        .into_iter()
        .rev()
        .map(|path| path.join(suffix))
        .find(|path| path.join(required).is_file())
}

pub(super) fn discover() -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    if let Some(lib) = std::env::var_os("LIB") {
        roots.extend(std::env::split_paths(&lib));
    }
    if let Some(vc) = std::env::var_os("VCToolsInstallDir") {
        roots.push(PathBuf::from(vc).join("lib/x64"));
    }
    let program_files = std::env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:/Program Files (x86)"));
    let vswhere = program_files.join("Microsoft Visual Studio/Installer/vswhere.exe");
    if let Ok(output) = Command::new(vswhere)
        .args([
            "-latest",
            "-products",
            "*",
            "-requires",
            "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
            "-property",
            "installationPath",
        ])
        .output()
    {
        if output.status.success() {
            let installation = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            if !installation.is_empty() {
                if let Some(path) = newest(
                    &PathBuf::from(installation).join("VC/Tools/MSVC"),
                    Path::new("lib/x64"),
                    "msvcrt.lib",
                ) {
                    roots.push(path);
                }
            }
        }
    }
    let sdk = std::env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .unwrap_or_else(|| program_files.join("Windows Kits/10"));
    if let Some(version) = std::env::var_os("WindowsSDKVersion") {
        roots.push(sdk.join("Lib").join(&version).join("ucrt/x64"));
        roots.push(sdk.join("Lib").join(version).join("um/x64"));
    }
    if let Some(ucrt) = newest(&sdk.join("Lib"), Path::new("ucrt/x64"), "ucrt.lib") {
        let version = ucrt.parent().unwrap().parent().unwrap();
        roots.push(version.join("um/x64"));
        roots.push(ucrt);
    }
    roots.retain(|root| root.is_dir());
    let mut unique = Vec::new();
    for root in roots {
        if !unique.contains(&root) {
            unique.push(root);
        }
    }
    Ok(unique)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_numeric_and_missing_libraries_are_skipped() {
        let root = tempfile::tempdir().unwrap();
        for version in ["14.9", "14.10", "14.11"] {
            std::fs::create_dir_all(root.path().join(version).join("lib/x64")).unwrap();
        }
        for version in ["14.9", "14.10"] {
            std::fs::write(root.path().join(version).join("lib/x64/msvcrt.lib"), "").unwrap();
        }
        assert_eq!(
            newest(root.path(), Path::new("lib/x64"), "msvcrt.lib").unwrap(),
            root.path().join("14.10/lib/x64")
        );
    }
}
