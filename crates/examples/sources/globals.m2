// globals: Global variables can call functions during initialization before main.
// Run: mlc run globals.m2
// Expected output: counter=42
import c_std::io;
func initial()->i32 { return 40; }
global var counter=initial();
func increment() { counter=counter+1; }
func main()->i32 {
    increment();
    increment();
    c_std::io::printf(const_c_str("counter=%d\n"),counter);
    return 0;
}
