//! Shared compiler distribution and backend configuration constants.

pub const EXECUTABLE_NAME: &str = "mlc";
/// Host executable suffix; independent of the compilation target.
pub const EXE_SUFFIX: &str = std::env::consts::EXE_SUFFIX;
pub const LIB_DIR: &str = "lib";
pub const UNIVERSAL_LIB_DIR: &str = "universal";
/// Linkers live beside the compiler executable in the distribution root.
pub const TOOLS_DIR: &str = ".";

pub const LLD_COFF: &str = "lld-link";
pub const LLD_ELF: &str = "ld.lld";
pub const IR_BUFFER_NAME: &std::ffi::CStr = c"mlc";

pub const X86_64_PC_WINDOWS_MSVC: &str = "x86_64-pc-windows-msvc";
pub const X86_64_PC_WINDOWS_GNU: &str = "x86_64-pc-windows-gnu";
pub const DEFAULT_WINDOWS_LINKER: &str = "x86_64-w64-mingw32-clang";

/// The compiler executable's platform, not the machine running Cargo.
/// Preserve architecture and Linux libc; Windows defaults to the GNU ABI.
pub fn system_triplet(target: &str) -> String {
    if target.contains("-windows-") {
        format!(
            "{}-pc-windows-gnu",
            target.split('-').next().unwrap_or("x86_64")
        )
    } else {
        target.to_owned()
    }
}

/// Resolve a linker in the compiler distribution, never through PATH.
pub fn lld_path(executable: &std::path::Path, coff: bool) -> Option<std::path::PathBuf> {
    Some(executable.parent()?.join(format!(
        "{}{}",
        if coff { LLD_COFF } else { LLD_ELF },
        EXE_SUFFIX
    )))
}

/// Current development installation, used by the Cargo build script.
pub const LLVM_INSTALL_DIR: &str = r"F:\Develop\scoop\apps\llvm-dev\current";
pub const LLVM_C_LIBRARY: &str = "LLVM-C";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_defaults_preserve_architecture_and_platform() {
        for (input, expected) in [
            ("x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu"),
            ("aarch64-pc-windows-msvc", "aarch64-pc-windows-gnu"),
            ("x86_64-pc-windows-gnu", "x86_64-pc-windows-gnu"),
            ("x86_64-unknown-linux-gnu", "x86_64-unknown-linux-gnu"),
            ("aarch64-unknown-linux-musl", "aarch64-unknown-linux-musl"),
            ("x86_64-apple-darwin", "x86_64-apple-darwin"),
            ("aarch64-apple-darwin", "aarch64-apple-darwin"),
        ] {
            assert_eq!(system_triplet(input), expected);
        }
    }

    #[test]
    fn linker_paths_are_executable_siblings() {
        let root = std::env::temp_dir().join("mlc-distribution");
        let executable = root.join(format!("{EXECUTABLE_NAME}{EXE_SUFFIX}"));
        assert_eq!(
            lld_path(&executable, true),
            Some(root.join(format!("{LLD_COFF}{EXE_SUFFIX}")))
        );
        assert_eq!(
            lld_path(&executable, false),
            Some(root.join(format!("{LLD_ELF}{EXE_SUFFIX}")))
        );
    }
}
