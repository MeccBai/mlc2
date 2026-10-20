// functions: Declare parameter and return types; compute Fibonacci recursively.
// Run: mlc run functions.m2
// Expected output (/ separates lines): fibonacci(10)=55
import std;
using println=std::println;

func fibonacci(n:i32) -> i32 {
    if (n == 0 || n == 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

func main() -> i32 {
    println!("fibonacci(10)={}", fibonacci(10));
    return 0;
}
