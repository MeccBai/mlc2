//! Formatting worker: no LLVM initialization or compilation driver.
#[allow(dead_code)]
mod cli;
#[path = "driver/formatting.rs"]
mod formatting;
mod output;
#[allow(dead_code)]
#[path = "driver/progress.rs"]
mod progress;
use progress::Progress;

fn main() {
    let cli = output::parse();
    let cli::Commands::Fmt { input } = cli.command else {
        std::process::exit(output::finish(
            "fmt",
            Err("mlc-fmt only accepts the fmt command".into()),
        ));
    };
    let progress = Progress::new(Some("Format"));
    let result = std::env::current_dir()
        .map_err(|error| mlc_core::diagnostic::Report::from(error.to_string()))
        .and_then(|root| formatting::run(&root, input.as_deref(), &progress));
    progress.finish(result.is_ok());
    std::process::exit(output::finish("fmt", result));
}
