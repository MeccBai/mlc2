//! Test executables use exactly the production MSVC linker argument table.
#[allow(dead_code)]
#[path = "../../src/driver/link/targets.rs"]
mod targets;
use mlc_builder::{
    manifest::{DEFAULT_WINDOWS_LINKER, X86_64_PC_WINDOWS_MSVC},
    project::TargetKind,
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub fn link(objects: &[PathBuf], executable: &Path) -> Output {
    let compiler = std::env::current_exe().unwrap();
    let directory = compiler.parent().unwrap();
    let installation = if directory.file_name().is_some_and(|name| name == "deps") {
        directory.parent().unwrap()
    } else {
        directory
    };
    let bundled = installation
        .join("tools")
        .join(format!("{DEFAULT_WINDOWS_LINKER}.exe"));
    let linker = std::env::var_os("MLC_TEST_LLD_LINK").unwrap_or_else(|| {
        if bundled.is_file() {
            bundled.into_os_string()
        } else {
            DEFAULT_WINDOWS_LINKER.into()
        }
    });
    let runtime = runtime(installation);
    let args = targets::profile(X86_64_PC_WINDOWS_MSVC)
        .unwrap()
        .arguments(&runtime, objects, executable, TargetKind::Bin)
        .unwrap();
    Command::new(linker)
        .args(args)
        .output()
        .expect("Install lld-link or set MLC_TEST_LLD_LINK")
}

fn runtime(installation: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("MLC_TEST_RUNTIME_DIR") {
        return path.into();
    }
    let bundled = installation.join("runtimes").join(X86_64_PC_WINDOWS_MSVC);
    if bundled.is_dir() {
        return bundled;
    }
    // Developer tests may run from target/debug rather than the installed MLC tree.
    // Reuse that installation's CRT if mlc.exe is already available on PATH.
    if let Some(path) = std::env::var_os("PATH") {
        for root in std::env::split_paths(&path) {
            let runtime = root.join("runtimes").join(X86_64_PC_WINDOWS_MSVC);
            if root.join("mlc.exe").is_file() && runtime.join("ucrt.lib").is_file() {
                return runtime;
            }
        }
    }
    bundled // The production profile can still discover VS/SDK libraries.
}

/// C is needed only for instrumentation fixtures, never as the linker driver.
#[allow(dead_code)]
pub fn compile_c(source: &Path, directory: &Path) -> PathBuf {
    let object = directory
        .join(source.file_stem().unwrap())
        .with_extension("obj");
    let compiler = std::env::var_os("MLC_TEST_CLANG_CL").unwrap_or_else(|| "clang-cl".into());
    let output = Command::new(compiler)
        .args([
            "--target=x86_64-pc-windows-msvc",
            "/nologo",
            "/c",
            "/MD",
            "/clang:-std=c11",
        ])
        .arg(source)
        .arg(format!("/Fo{}", object.display()))
        .output()
        .expect("Install clang-cl or set MLC_TEST_CLANG_CL to compile C fixtures");
    assert!(
        output.status.success(),
        "C fixture {}:\n{}\n{}",
        source.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    object
}
