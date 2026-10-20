// memory: alloc returns uninitialized storage; write before reading; dealloc consumes ownership.
// Run: mlc run memory.m2
// Expected output (/ separates lines): allocated value=42
import std::mem;
import std;
using println=std::println;

func main() -> i32 {
    // Raw, uninitialized storage owned by a resource with automatic RAII cleanup.
    var pointer:res $mut i32 = std::mem::alloc<i32>(1);
    // Allocation failure terminates the program without unwinding.
    $pointer = 42;
    println!("allocated value={}", $pointer);
    std::mem::dealloc<i32>(pointer);
    // Do not use pointer again after deallocation.
    return 0;
}
