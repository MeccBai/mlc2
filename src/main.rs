mod cli;
mod driver;
pub mod manifest;

fn main() -> std::process::ExitCode {
    use clap::Parser;
    match driver::run(cli::Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
