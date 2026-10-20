// variables: var is mutable, val cannot be rebound, const requires a constant; cast permits truncation.
// Run: mlc run variables.m2
// Expected output (/ separates lines): answer=42 counter=43 narrowed=44 / fraction=1.5
// var: mutable; val: immutable; const: compile-time constant.
import std;
using println=std::println;

func main() -> i32 {
    const base:i32 = 6 * 7;
    val answer = base;
    var counter = answer;
    counter = counter + 1;

    // Explicit cast permits truncation; implicit i8 = 300 is an error.
    val narrowed:i8 = cast<i8>(300);
    println!("answer={} counter={} narrowed={}",
        answer, counter, cast<i32>(narrowed));
    // Floating literals are f64; narrowing to f32 also requires an explicit cast.
    val fraction:f32=cast<f32>(1.5);
    println!("fraction={}",fraction);
    return 0;
}
