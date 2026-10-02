//! Shared compiler distribution and backend configuration constants.

pub const EXECUTABLE_NAME: &str = "mlc";
/// Host executable suffix; independent of the compilation target.
pub const EXE_SUFFIX: &str = std::env::consts::EXE_SUFFIX;
pub const SOURCE_SUFFIX: &str = ".vl";
pub const LIB_DIR: &str = "lib";
pub const TOOLS_DIR: &str = "tools";

pub const LLD_COFF: &str = "lld-link";
pub const LLD_ELF: &str = "ld.lld";
pub const IR_BUFFER_NAME: &std::ffi::CStr = c"mlc";
pub const IR_RESERVE_SIZE: usize = 2 * 1000 * 1000;
pub const ABI_DIRECT_SIZE_LIMIT: usize = 8;

pub const X86_64_PC_WINDOWS_MSVC: &str = "x86_64-pc-windows-msvc";
pub const THUMBV7EM_NONE_EABI: &str = "thumbv7em-none-eabi";
pub const AARCH64_NONE_ELF: &str = "aarch64-none-elf";
pub const RISCV32_NONE_ELF: &str = "riscv32-unknown-none-elf";

/// Current development installation, used by the Cargo build script.
pub const LLVM_INSTALL_DIR: &str = r"F:\Develop\scoop\apps\llvm-dev\current";
pub const LLVM_C_LIBRARY: &str = "LLVM-C";
