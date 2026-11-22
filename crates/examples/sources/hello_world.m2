// hello_world: Import the standard library and print a line with println!.
// Run: mlc run hello_world.m2
// Expected output (/ separates lines): Hello World!
import std;

using println = std::println;

func main() -> i32 {
    var greeting: str = "Hello World!"; // str bindings are read-only by default.
    println!("{}", greeting);
    return 0;
}
