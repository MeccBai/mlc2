// hello_world: Learn main, import and const_c_str, which creates a NUL-terminated C string.
// Run: mlc run hello_world.m2
// Expected output (/ separates lines): Hello World!
import c_std::io;

func main() -> i32 {
    c_std::io::printf(const_c_str("Hello World!\n"));
    return 0;
}
