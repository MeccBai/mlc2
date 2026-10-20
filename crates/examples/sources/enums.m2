// enums: Qualify enum values with their type name; enum values carry no payload.
// Run: mlc run enums.m2
// Expected output: running
import std;
using println=std::println;
enum State { Waiting,Running,Stopped };
func main()->i32 {
    val state=State::Running;
    match (state) {
        State::Waiting => { println!("waiting"); },
        State::Running => { println!("running"); },
        _ => { println!("stopped"); }
    }
    return 0;
}
