// pipelines: {2,5} supplies two arguments to add; its result becomes the argument to twice.
// Run: mlc run pipelines.m2
// Expected output: result=14
import std;
using println=std::println;
func add(a:i32,b:i32)->i32 { return a+b; }
func twice(value:i32)->i32 { return value*2; }
func main()->i32 {
    val result={2,5} |> add |> twice; // Equivalent to twice(add(2,5)).
    println!("result={}",result);
    return 0;
}
