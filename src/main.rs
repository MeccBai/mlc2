#[allow(dead_code)]
mod cli;
mod dispatch;
mod output;
fn main() {
    let cli = output::parse();
    let command = output::command(&cli);
    let code = match dispatch::run(cli) {
        Ok(code) => code,
        Err(error) => output::finish(command, Err(error.into())),
    };
    std::process::exit(code);
}
