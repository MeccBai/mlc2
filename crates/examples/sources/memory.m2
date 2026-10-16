import std::mem;
import c_std::io;

func main() -> i32 {
    // Raw, uninitialized storage owned by a resource with automatic RAII cleanup.
    var pointer:res $mut i32 = std::mem::alloc<i32>(1);
    // Allocation failure terminates the program without unwinding.
    $pointer = 42;
    c_std::io::printf(const_c_str("allocated value=%d\n"), $pointer);
    std::mem::dealloc<i32>(pointer);
    // Do not use pointer again after deallocation.
    return 0;
}
