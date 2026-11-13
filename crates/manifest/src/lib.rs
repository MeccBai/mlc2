//! Shared compiler configuration, target platform policy, and distribution constants.

pub const EXECUTABLE_NAME: &str = "mlc";
/// Artifact/cache compatibility identity. Increment when compiler semantics or
/// serialized layouts change; the display/package version is not a cache key.
pub const BUILD_NUMBER: u32 = 3;
/// Host executable suffix; independent of the compilation target.
pub const EXE_SUFFIX: &str = std::env::consts::EXE_SUFFIX;
pub const LIB_DIR: &str = "lib";
pub const OBJECTS_DIR: &str = "objects";
pub const RUNTIMES_DIR: &str = "runtimes";
pub const UNIVERSAL_LIB_DIR: &str = "universal";
/// Executable tools are isolated from compiler and library files.
pub const TOOLS_DIR: &str = "tools";

pub const IR_BUFFER_NAME: &std::ffi::CStr = c"mlc";

pub const X86_64_PC_WINDOWS_MSVC: &str = "x86_64-pc-windows-msvc";
pub const X86_64_PC_WINDOWS_GNU: &str = "x86_64-pc-windows-gnu";

mod abi;
mod arch;
mod system;

pub use abi::Abi;
pub use arch::Arch;
pub use arch::Device;
pub use system::System;

/// Architecture and system come from compile-time cfg; ABI policy lives in System.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    arch: Arch,
    system: System,
    abi: Abi,
}

impl Default for Target {
    fn default() -> Self {
        Self::host()
    }
}

impl Target {
    /// Override the architecture while retaining the default platform policy.
    pub fn with_arch(self, arch: Arch) -> Self {
        Self::new(arch, self.system, Some(self.abi)).expect("unsupported target architecture")
    }

    /// Selecting a system also selects that system's built-in default ABI.
    pub fn with_system(self, system: System) -> Self {
        Self::new(self.arch, system, None).expect("unsupported target system")
    }

    pub fn with_abi(self, abi: Abi) -> Self {
        Self::new(self.arch, self.system, Some(abi)).expect("unsupported target ABI")
    }

    pub const fn ptr_size(&self) -> usize {
        match self.arch {
            Arch::X86_64 | Arch::Aarch64 => 8,
            Arch::Arm
            | Arch::Armv7
            | Arch::Armv7R
            | Arch::Armv8
            | Arch::Thumbv6m
            | Arch::Thumbv7a
            | Arch::Thumbv7em
            | Arch::Thumbv7m
            | Arch::Thumbv8m
            | Arch::X86 => 4,
        }
    }

    pub const fn arch(self) -> Arch {
        self.arch
    }
    pub const fn system(self) -> System {
        self.system
    }
    pub const fn abi(self) -> Abi {
        self.abi
    }

    pub fn new(arch: Arch, system: System, abi: Option<Abi>) -> Result<Self, String> {
        let abi = abi.unwrap_or_else(|| system.default_abi());
        if !system.supports(abi) {
            return Err(format!("unsupported ABI {abi:?} for {system:?}"));
        }
        // These are the complete desktop triples supported by this manifest.
        if !matches!(arch, Arch::X86 | Arch::X86_64 | Arch::Aarch64)
            || (system == System::Macos && arch == Arch::X86)
        {
            return Err(format!("unsupported architecture {arch:?} for {system:?}"));
        }
        Ok(Self { arch, system, abi })
    }

    pub const fn host() -> Self {
        let system = System::host();
        Self {
            arch: Arch::host(),
            system,
            abi: system.default_abi(),
        }
    }

    pub const fn triplet(self) -> &'static str {
        macro_rules! triple {
            ($arch:literal) => {
                match (self.system, self.abi) {
                    (System::Windows, Abi::Msvc) => concat!($arch, "-pc-windows-msvc"),
                    (System::Windows, Abi::Gnu) => concat!($arch, "-pc-windows-gnu"),
                    (System::Linux, Abi::Gnu) => concat!($arch, "-unknown-linux-gnu"),
                    (System::Linux, Abi::Musl) => concat!($arch, "-unknown-linux-musl"),
                    (System::Macos, Abi::Darwin) => concat!($arch, "-apple-darwin"),
                    _ => panic!("unsupported system/ABI combination"),
                }
            };
        }
        match self.arch {
            Arch::X86_64 => triple!("x86_64"),
            Arch::X86 => triple!("i686"),
            Arch::Aarch64 => triple!("aarch64"),
            _ => panic!("unsupported target architecture"),
        }
    }
    pub const fn linker(self) -> &'static str {
        self.abi.linker()
    }

    /// The linker targets this ABI but runs on the compiler's host platform.
    pub fn linker_path(self, executable: &std::path::Path) -> Option<std::path::PathBuf> {
        Some(
            executable
                .parent()?
                .join(TOOLS_DIR)
                .join(format!("{}{}", self.linker(), EXE_SUFFIX)),
        )
    }
}

pub const LLVM_C_LIBRARY: &str = "LLVM-C";

pub const SOURCE_SUFFIX: &str = ".m2";

pub const IR_RESERVE_SIZE: usize = 2 * 1000 * 1000;
pub const ABI_DIRECT_SIZE_LIMIT: usize = 8;
