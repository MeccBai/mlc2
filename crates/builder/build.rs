use mlc_manifest as manifest;
use std::{path::PathBuf, process::Command};

fn output(program: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{}: {e}", program.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{}: {}",
            program.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| e.to_string())
}

fn main() {
    for variable in ["SCOOP", "LLVM_CONFIG", "LLVM_SYS_231_PREFIX"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let target = std::env::var("TARGET").expect("Cargo provides TARGET");

    if target.contains("windows") {
        let prefix = std::env::var_os("LLVM_SYS_231_PREFIX")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("SCOOP")
                    .map(|root| PathBuf::from(root).join("apps/llvm-dev/current"))
            })
            .expect("Set LLVM_SYS_231_PREFIX to LLVM 23, or install llvm-dev with Scoop");
        let lib = prefix.join(manifest::LIB_DIR);
        if !lib.join("LLVM-C.lib").is_file() {
            panic!("LLVM-C.lib was not found under {}", lib.display());
        }
        println!("cargo:rustc-link-search=native={}", lib.display());
        println!("cargo:rustc-link-lib=dylib={}", manifest::LLVM_C_LIBRARY);
        return;
    }

    let candidates = if let Some(config) = std::env::var_os("LLVM_CONFIG") {
        vec![PathBuf::from(config)]
    } else if let Some(prefix) = std::env::var_os("LLVM_SYS_231_PREFIX") {
        vec![PathBuf::from(prefix).join("bin/llvm-config")]
    } else {
        vec![
            PathBuf::from("llvm-config-23"),
            PathBuf::from("llvm-config"),
        ]
    };
    let mut failures = Vec::new();

    let config = candidates
        .into_iter()
        .find(|candidate| match output(candidate, &["--version"]) {
            Ok(version) if version.split('.').next() == Some("23") => true,
            Ok(version) => {
                failures.push(format!(
                    "{} reports LLVM {version}; expected LLVM 23",
                    candidate.display()
                ));
                false
            }
            Err(error) => {
                failures.push(error);
                false
            }
        })
        .unwrap_or_else(|| {
            panic!(
                "Cannot locate LLVM 23. Set LLVM_CONFIG or LLVM_SYS_231_PREFIX.\n{}",
                failures.join("\n")
            )
        });

    let directory = output(&config, &["--libdir"]).unwrap_or_else(|error| panic!("{error}"));

    let libraries = output(&config, &["--libs", "--link-shared"])
        .unwrap_or_else(|error| panic!("A shared LLVM 23 library is required: {error}"));
    println!("cargo:rustc-link-search=native={directory}");

    let mut linked = false;

    for name in libraries
        .split_whitespace()
        .filter_map(|arg| arg.strip_prefix("-l"))
    {
        println!("cargo:rustc-link-lib=dylib={name}");
        linked = true;
    }
    assert!(
        linked,
        "llvm-config did not report shared LLVM libraries: {libraries}"
    );
}
