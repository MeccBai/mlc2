import c_std::io;

func main() -> i32 {
    var text:[i8:14] = "Hello World!\n";
    text[13] = 0;
    c_std::io::printf(@text[0]);
    return 0;
}
