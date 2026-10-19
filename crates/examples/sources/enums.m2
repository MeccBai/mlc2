// enums: Qualify enum values with their type name; enum values carry no payload.
// Run: mlc run enums.m2
// Expected output: running
import c_std::io;
enum State { Waiting,Running,Stopped };
func main()->i32 {
    val state=State::Running;
    match (state) {
        State::Waiting => { c_std::io::printf(const_c_str("waiting\n")); },
        State::Running => { c_std::io::printf(const_c_str("running\n")); },
        _ => { c_std::io::printf(const_c_str("stopped\n")); }
    }
    return 0;
}
