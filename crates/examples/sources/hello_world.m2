// hello_world: Import the standard library and print a line with println!.
// Run: mlc run hello_world.m2
// Expected output (/ separates lines): Hello World!
import std;
using println=std::println;

func main() -> i32 {
    val greeting:str="Hello World!"; // str is an immutable compile-time string.
    println!("{}",greeting);
    return 0;
}
