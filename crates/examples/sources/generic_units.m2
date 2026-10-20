// generic_units: Construct Box with explicit type arguments; read infers T from Box<i32>.
// Run: mlc run generic_units.m2
// Expected output: value=42
import std;
using println=std::println;
unit<T> Box { pub value:T; };
func<T> read(boxed:Box<T>)->T { return boxed.value; }
func main()->i32 {
    val boxed=Box<i32>{42};
    println!("value={}",read(boxed));
    return 0;
}
