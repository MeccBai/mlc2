mod cli;
mod driver;
pub mod manifest;

fn main() {
    use clap::Parser;
    match driver::run(cli::Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1)
        }
    }
}
