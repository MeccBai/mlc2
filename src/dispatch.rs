//! Route commands to sibling executables without initializing a compiler.
use crate::cli::{Cli, Commands};
use std::process::Command;

pub fn run(cli: Cli) -> Result<i32, String> {
    let worker = match cli.command {
        Commands::Fmt { .. } => "mlc-fmt",
        _ => "mlc-build",
    };
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let path = executable
        .parent()
        .ok_or("Compiler has no installation directory")?
        .join(format!("{worker}{}", std::env::consts::EXE_SUFFIX));
    if !path.is_file() {
        return Err(format!(
            "Command worker not found: {}\nInstall mlc, mlc-build and mlc-fmt together; for source builds use cargo build --bins.",
            path.display()
        ));
    }
    Command::new(&path)
        .args(std::env::args_os().skip(1))
        .status()
        .map_err(|error| format!("Cannot start {}: {error}", path.display()))?
        .code()
        .ok_or_else(|| format!("{worker} terminated without an exit code"))
}
