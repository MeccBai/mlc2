//! Compiler worker: this executable owns the LLVM-C runtime dependency.
mod cli;
mod driver;
pub mod manifest;

fn main() {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "__mlc_ctfe_worker")
    {
        if let Err(error) = driver::constant_worker() {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    use clap::Parser;
    match driver::run(cli::Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
