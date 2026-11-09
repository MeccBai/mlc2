// const-functions: Evaluate loops, recursion and nested calls during compilation.
// Run: mlc run const_functions.m2
// Expected output (/ separates lines): sum_squares(5)=30 factorial(5)=120 / runtime value=7
import std;

using println = std::println;

// Const functions can read global const bindings, but cannot access global var or val.
const func square(value: i32) -> i32 {
    return value * value;
}


const func sum_squares(count: i32) -> i32 {
    var index = 0;
    var total = 0;
    while (index < count) {
        total += square(index);
        index += 1;
    }
    return total;
}


const func factorial(value: i32) -> i32 {
    if (value <= 1) {
        return 1;
    }
    return value * factorial(value - 1);
}


func runtime_value() -> i32 {
    return 7;
}


func main() -> i32 {
    // These calls run during compilation, including in the default build profile.
    const sum = sum_squares(5);
    const product = factorial(5);
    // val accepts a runtime initializer and makes the binding immutable.
    val runtime = runtime_value();
    // const invalid = runtime_value(); // Error: a const initializer needs a compile-time value.
    println!(
        "sum_squares(5)={} factorial(5)={}",
        sum,
        product
    );
    println!("runtime value={}", runtime);
    return 0;
}
