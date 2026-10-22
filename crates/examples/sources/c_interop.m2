// c_interop: C declarations use #[c_abi]#; C strings require NUL termination.
// Run: mlc run c_interop.m2
// Expected output (/ separates lines): C printf: 42 / byte=65
import c_std::io;
import std;

#[c_abi]#
func abs(value: i32) -> i32;

func main() -> i32 {
    // printf uses C format directives; it does not understand {} placeholders.
    c_std::io::printf(
        const_c_str("C printf: %d\n"),
        abs(- 42)
    );
    var bytes: [i8: 2] = [65, 0];
    std::println!("byte={}", bytes[0]);
    return 0;
}
