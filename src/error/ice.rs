pub fn ice(reason: &str) -> ! {
    println!("Internal Compiler Error: \n{}", reason);
    panic!()
}
