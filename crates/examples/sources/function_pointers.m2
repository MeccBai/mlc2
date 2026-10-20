// function_pointers: Pass a function pointer as an ordinary parameter.
// Run: mlc run function_pointers.m2
// Expected output: answer=42
import std;
using println=std::println;
func twice(value:i32)->i32 { return value*2; }
func apply(callback:func(i32)->i32,value:i32)->i32 { return callback(value); }
func main()->i32 {
    val callback=std::function(twice); // Obtain a function pointer using the builtin.
    println!("answer={}",apply(callback,21));
    return 0;
}
