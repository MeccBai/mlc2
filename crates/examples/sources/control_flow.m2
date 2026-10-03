import c_std::io;

func main() -> i32 {
    var sum = 0;
    // Numeric loop: 0 inclusive, 5 exclusive.
    for i in [0, 5] {
        sum = sum + i;
    }
    if (sum == 10) {
        c_std::io::printf(const_c_str("sum=%d\n"), sum);
    } else {
        return 1;
    }
    match (sum) {
        10 => { c_std::io::printf(const_c_str("matched ten\n")); },
        _ => { return 2; }
    }
    return 0;
}
