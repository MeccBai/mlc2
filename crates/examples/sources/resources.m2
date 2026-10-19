// resources: References borrow; res parameters take ownership and release it on exit.
// Run: mlc run resources.m2
// Expected output: observed=21 result=42
import std::mem;
import c_std::io;
func consume(memory:res $mut i32)->i32 {
    $memory=$memory*2;
    return $memory; // Release the resource after copying the return value.
}
func main()->i32 {
    var owner:res=std::mem::alloc<i32>(1);
    $owner=21;
    var observed=0;
    anonymous {
        val view=owner; // Without res, this borrows rather than transferring ownership.
        observed=$view;
    } // Move owner only after the borrow ends.
    val result=consume(owner);
    // owner has moved; do not read or release it again.
    c_std::io::printf(const_c_str("observed=%d result=%d\n"),observed,result);
    return 0;
}
