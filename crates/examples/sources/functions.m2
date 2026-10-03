import c_std::io;

func fibonacci(n:i32) -> i32 {
    if (n == 0 || n == 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

func main() -> i32 {
    c_std::io::printf(const_c_str("fibonacci(10)=%d\n"), fibonacci(10));
    return 0;
}
