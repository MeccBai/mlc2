use colored::Colorize;

pub fn ice(reason: &str) -> ! {
    if RECOVERABLE.with(|state| state.get()) {
        panic!("Internal Compiler Error: {reason}");
    }
    eprintln!("{} \n{}", "Internal Compiler Error:".red(), reason);
    std::process::abort();
}

thread_local! {static RECOVERABLE:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub struct Recovery(bool);
impl Drop for Recovery {
    fn drop(&mut self) {
        RECOVERABLE.with(|state| state.set(self.0));
    }
}
/// Editor requests may discard a failed analysis without terminating the server.
pub fn recoverable() -> Recovery {
    Recovery(RECOVERABLE.with(|state| state.replace(true)))
}
