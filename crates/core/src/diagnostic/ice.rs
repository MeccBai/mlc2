use colored::Colorize;

pub fn ice(reason: &str) -> ! {
    eprintln!("{} \n{}", "Internal Compiler Error:".red(), reason);
    std::process::abort();
}
