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
