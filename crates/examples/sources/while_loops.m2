// while_loops: Sum the odd numbers below 6.
// Run: mlc run while_loops.m2
// Expected output: sum=9
import c_std::io;
func main()->i32 {
    var i=0;
    var sum=0;
    while (i<10) {
        i=i+1;
        if (i==6) { break; }       // Exit the loop.
        if (i%2==0) { continue; } // Skip the rest of this iteration.
        sum=sum+i;
    }
    c_std::io::printf(const_c_str("sum=%d\n"),sum);
    return 0;
}
