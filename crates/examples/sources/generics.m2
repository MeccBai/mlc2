import c_std::io;

func<T> swap(a:$mut T, b:$mut T) {
    var temporary = $a;
    $a = $b;
    $b = temporary;
}

func main() -> i32 {
    var a = 10;
    var b = 20;
    swap<i32>(@mut a, @mut b);
    c_std::io::printf(const_c_str("a=%d b=%d\n"), a, b);
    return 0;
}
