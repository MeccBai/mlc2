use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(version, about = "MLC compiler and project builder")]
pub struct Cli {
    /// Build without optimization or debugging information.
    #[arg(long = "default", global = true, conflicts_with_all = ["debug", "release", "rel_with_info"])]
    pub default_mode: bool,
    /// Build without optimization, with debugging information.
    #[arg(long, global = true, conflicts_with_all = ["release", "rel_with_info"])]
    pub debug: bool,
    /// Build with optimization, without debugging information.
    #[arg(long, global = true)]
    pub release: bool,
    /// Build with optimization and debugging information.
    #[arg(long, global = true, conflicts_with = "release")]
    pub rel_with_info: bool,
    /// Override the mode to enable AST and LLVM code-generation optimizations.
    #[arg(long, global = true, conflicts_with = "no_optimize")]
    pub optimize: bool,
    /// Override the mode to disable optional AST and LLVM optimizations.
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
    /// Convert a restricted C header or declaration TOML to a binary .sym table.
    Symbols {
        input: PathBuf,
        /// Declaration module name; defaults to the input filename stem.
        #[arg(long)]
        module: Option<String>,
    },
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

impl Cli {
    pub fn build_mode(
        &self,
        configured: mlc_builder::project::BuildMode,
    ) -> mlc_builder::project::BuildMode {
        use mlc_builder::project::BuildMode;
        if self.default_mode {
            BuildMode::Default
        } else if self.debug {
            BuildMode::Debug
        } else if self.release {
            BuildMode::Release
        } else if self.rel_with_info {
            BuildMode::RelWithInfo
        } else {
            configured
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlc_builder::project::BuildMode;
    #[test]
    fn four_modes_override_configuration_and_are_mutually_exclusive() {
        let modes = [
            ("--default", BuildMode::Default),
            ("--debug", BuildMode::Debug),
            ("--release", BuildMode::Release),
            ("--rel-with-info", BuildMode::RelWithInfo),
        ];
        let default = Cli::try_parse_from(["mlc", "build", "main.m2"]).unwrap();
        assert_eq!(default.build_mode(BuildMode::default()), BuildMode::Default);
        assert_eq!(
            default.build_mode(BuildMode::RelWithInfo),
            BuildMode::RelWithInfo
        );
        for (flag, mode) in modes {
            let cli = Cli::try_parse_from(["mlc", "build", "main.m2", flag]).unwrap();
            assert_eq!(cli.build_mode(BuildMode::Debug), mode);
            for (other, _) in modes {
                if other != flag {
                    assert!(Cli::try_parse_from(["mlc", "build", "main.m2", flag, other]).is_err());
                }
            }
        }
    }
}
