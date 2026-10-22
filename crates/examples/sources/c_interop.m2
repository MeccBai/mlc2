// c_interop: C declarations are imported from .sym; C strings require NUL termination.
// Run: mlc run c_interop.m2
// Expected output (/ separates lines): C printf: 42 / byte=65
import c_std::io;
import c_std::runtime;
import std;

func main() -> i32 {
    // printf uses C format directives; it does not understand {} placeholders.
    c_std::io::printf(
        const_c_str("C printf: %d\n"),
        c_std::runtime::abs(- 42)
    );
    var bytes: [i8: 2] = [65, 0];
    std::println!("byte={}", bytes[0]);
    return 0;
}
