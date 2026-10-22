// aliases: Alias a type, a function, and a standard-library macro.
// Run: mlc run aliases.m2
// Expected output: answer=42
import std;


using println = std::println;
using Number = i32;
func twice(value: Number) -> Number {
    return value * 2;
}


using double = twice;
func main() -> i32 {
    println!("answer={}", double(21));
    return 0;
}
