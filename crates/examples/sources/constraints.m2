// constraints: Constraints are checked after type inference or explicit instantiation.
// Run: mlc run constraints.m2
// Expected output: answer=42
import c_std::io;
generic Integer { std::generic::is_integer; };
func<T:Integer> twice(value:T)->T { return value+value; }
func main()->i32 {
    val answer=twice(21);
    // twice(1.5) fails: f64 does not satisfy Integer.
    c_std::io::printf(const_c_str("answer=%d\n"),answer);
    return 0;
}
