use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about = "MLC compiler and project builder")]
pub struct Cli {
    /// Enable all AST optimization passes.
    #[arg(long, global = true, conflicts_with = "no_optimize")]
    pub optimize: bool,
    /// Disable optional AST optimizations (semantic const checks remain enabled).
    #[arg(long, global = true)]
    pub no_optimize: bool,
    #[command(subcommand)]
    pub command: Commands,
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
    /// Override the triplet's bundled linker executable.
    #[arg(long, global = true)]
    pub linker: Option<PathBuf>,
    /// Override tools/llvm-ar in the compiler installation.
    #[arg(long, global = true)]
    pub archiver: Option<PathBuf>,
    /// Extra native linker argument (use --link-arg=-lfoo for options starting with '-').
    #[arg(long = "link-arg", global = true)]
    pub link_args: Vec<String>,
    #[arg(short = 'j', long, global = true)]
    pub jobs: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Format one source file, or all project sources when no input is given.
    Fmt { input: Option<PathBuf> },
    /// Generate LLVM IR (.ll) only, without object emission or linking.
    Ll { input: PathBuf },
    /// Convert a hand-written declaration TOML to a binary .sym symbol table.
    Symbols { input: PathBuf },
    /// List bundled examples or print one example's source code.
    Example { name: String },
    /// Build a .m2 entry file (ignoring Project.toml), a named target, or all targets.
    Build { target: Option<String> },
    /// Build and run a .m2 entry file or a named executable target.
    Run { target: String },
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
