// references: @ borrows read-only, @mut borrows mutably, and $ accesses the referenced value.
// Run: mlc run references.m2
// Expected output (/ separates lines): value=42
import std;
using println=std::println;

func increment(value:$mut i32) {
    $value = $value + 1;
}

func main() -> i32 {
    var value = 41;
    increment(@mut value);
    val view = @value;
    println!("value={}", $view);
    return 0;
}
