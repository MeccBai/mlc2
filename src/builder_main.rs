//! Compiler worker: this executable owns the LLVM-C runtime dependency.
mod cli;
mod driver;
pub mod manifest;
mod output;

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
    let cli = output::parse();
    let command = output::command(&cli);
    let result = driver::run(cli);
    std::process::exit(output::finish(command, result));
}
