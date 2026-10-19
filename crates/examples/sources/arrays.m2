// arrays: [Type:length] declares an array; reserve an extra byte for the C string terminator.
// Run: mlc run arrays.m2
// Expected output (/ separates lines): Hello World! / numbers[1]=25
import c_std::io;

func main() -> i32 {
    // Arrays use [], not {}. Partial initialization warns and zero-fills.
    var numbers:[i32:4] = [10, 20, 30, 40];
    numbers[1] = 25;

    // Ordinary string arrays need capacity for a NUL terminator.
    var text:[i8:14] = "Hello World!\n";
    text[13] = 0;
    c_std::io::printf(@text[0]);
    c_std::io::printf(const_c_str("numbers[1]=%d\n"), numbers[1]);
    return 0;
}
