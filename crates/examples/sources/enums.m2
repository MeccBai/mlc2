// enums: Qualify enum values with their type name; each value has an explicitly typed constant representation.
// Run: mlc run enums.m2
// Expected output: running
import std;

using println = std::println;

enum State(i32) {
    Waiting(0),
    Running(1),
    Stopped(2)
};
func main() -> i32 {
    val state = State::Running;
    val raw = state.value(); // i32: 1. No runtime call.
    match (state) {
        State::Waiting => {
            println!("waiting");
        },
        State::Running => {
            println!("running");
        },
        State::Stopped => {
            println!("stopped");
        }
    }
    return 0;
}
