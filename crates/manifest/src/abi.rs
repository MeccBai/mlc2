#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Abi {
    Msvc,
    Gnu,
    Musl,
    Darwin,
}

impl Abi {
    pub const fn linker(self) -> &'static str {
        match self {
            Self::Msvc => "lld-link",
            Self::Gnu | Self::Musl => "ld.lld",
            Self::Darwin => "ld64.lld",
        }
    }
}
