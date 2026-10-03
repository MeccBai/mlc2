use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about = "MLC compiler and project builder")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Source entry file (without a subcommand).
    pub entry: Option<PathBuf>,
    /// Build output directory.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,
    #[arg(long, global = true)]
    pub triplet: Option<String>,
    #[arg(long = "type", value_enum, global = true)]
    pub kind: Option<Kind>,
    /// Extra library root; triplet is appended automatically.
    #[arg(long = "lib-dir", global = true)]
    pub lib_dirs: Vec<PathBuf>,
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    /// Clang linker driver, providing the platform CRT/SDK paths.
    #[arg(long, default_value = "clang", global = true)]
    pub linker: PathBuf,
    #[arg(long, default_value = "llvm-ar", global = true)]
    pub archiver: PathBuf,
    /// Extra driver argument (use --link-arg=-lfoo for options starting with '-').
    #[arg(long = "link-arg", global = true)]
    pub link_args: Vec<String>,
    #[arg(short = 'j', long, global = true)]
    pub jobs: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Convert a hand-written declaration TOML to a binary .sym symbol table.
    Symbols { input: PathBuf },
    /// List bundled examples or print one example's source code.
    Example { name: String },
    /// Build one named target, or all targets in Project.toml.
    Build { target: Option<String> },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Kind {
    Bin,
    Static,
    Shared,
}

impl From<Kind> for mlc_builder::project::TargetKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Bin => Self::Bin,
            Kind::Static => Self::Static,
            Kind::Shared => Self::Shared,
        }
    }
}
