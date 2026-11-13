use super::Abi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Windows,
    Linux,
    Macos,
}

impl System {
    pub const fn host() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Self::Macos
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        compile_error!("unsupported compiler system");
    }
    pub const fn default_abi(self) -> Abi {
        match self {
            Self::Windows => Abi::Msvc,
            Self::Linux => Abi::Gnu,
            Self::Macos => Abi::Darwin,
        }
    }

    pub const fn supports(self, abi: Abi) -> bool {
        matches!(
            (self, abi),
            (Self::Windows, Abi::Msvc | Abi::Gnu)
                | (Self::Linux, Abi::Gnu | Abi::Musl)
                | (Self::Macos, Abi::Darwin)
        )
    }
}
