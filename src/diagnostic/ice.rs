use colored::Colorize;

pub fn ice(reason: &str) -> ! {
    println!("{} \n{}", "Internal Compiler Error:".red(), reason);
    panic!()
}
