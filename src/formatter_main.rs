//! Formatting worker: no LLVM initialization or compilation driver.
#[allow(dead_code)]
mod cli;
#[path = "driver/formatting.rs"]
mod formatting;
#[allow(dead_code)]
#[path = "driver/progress.rs"]
mod progress;
use progress::Progress;

fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    let cli::Commands::Fmt { input } = cli.command else {
        eprintln!("mlc-fmt only accepts the fmt command");
        std::process::exit(1);
    };
    let progress = Progress::new(Some("Format"));
    let result = std::env::current_dir()
        .map_err(|error| error.to_string())
        .and_then(|root| formatting::run(&root, input.as_deref(), &progress));
    progress.finish(result.is_ok());
    match result {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
