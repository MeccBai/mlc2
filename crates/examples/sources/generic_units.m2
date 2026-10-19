// generic_units: Construct Box with explicit type arguments; read infers T from Box<i32>.
// Run: mlc run generic_units.m2
// Expected output: value=42
import c_std::io;
unit<T> Box { pub value:T; };
func<T> read(boxed:Box<T>)->T { return boxed.value; }
func main()->i32 {
    val boxed=Box<i32>{42};
    c_std::io::printf(const_c_str("value=%d\n"),read(boxed));
    return 0;
}
