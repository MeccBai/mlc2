import c_std::io;

func increment(value:$mut i32) {
    $value = $value + 1;
}

func main() -> i32 {
    var value = 41;
    increment(@mut value);
    val view = @value;
    c_std::io::printf(const_c_str("value=%d\n"), $view);
    return 0;
}
