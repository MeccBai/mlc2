#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    X86,
    Aarch64,
    Arm,
    Armv7,
    Armv7R,
    Armv8,
    Thumbv6m,
    Thumbv7a,
    Thumbv7em,
    Thumbv7m,
    Thumbv8m,
}

impl Arch {
    pub const fn host() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }
        #[cfg(target_arch = "x86")]
        {
            Self::X86
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }
        #[cfg(target_arch = "arm")]
        {
            Self::Arm
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        compile_error!("unsupported compiler architecture");
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::X86 => "i686",
            Self::Aarch64 => "aarch64",
            Self::Arm => "arm",
            Self::Armv7 => "armv7",
            Self::Armv7R => "armv7r",
            Self::Armv8 => "armv8",
            Self::Thumbv6m => "thumbv6m",
            Self::Thumbv7a => "thumbv7a",
            Self::Thumbv7em => "thumbv7em",
            Self::Thumbv7m => "thumbv7m",
            Self::Thumbv8m => "thumbv8m",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Pc,
    Unknown,
    None,
    Apple,
}
