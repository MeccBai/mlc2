#[allow(dead_code)]
mod cli;
mod dispatch;

fn main() {
    use clap::Parser;
    match dispatch::run(cli::Cli::parse()) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1)
        }
    }
}
