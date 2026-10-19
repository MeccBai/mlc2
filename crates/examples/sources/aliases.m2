// aliases: using creates a symbol alias, not a new runtime object.
// Run: mlc run aliases.m2
// Expected output: answer=42
import c_std::io;
using Number=i32;
using print=c_std::io::printf;
func twice(value:Number)->Number { return value*2; }
func main()->i32 {
    print(const_c_str("answer=%d\n"),twice(21));
    return 0;
}
