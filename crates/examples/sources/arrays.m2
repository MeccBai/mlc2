// arrays: Fixed-size arrays, indexing, and an explicit non-owning string view.
// Run: mlc run arrays.m2
// Expected output (/ separates lines): Hello World! / numbers[1]=25
import std;


using println = std::println;
func main() -> i32 {
    // Arrays use [], not {}. Partial initialization warns and zero-fills.
    var numbers: [i32: 4] = [10, 20, 30, 40];
    numbers[1] = 25;
    // The view stores a byte length; it does not scan for a NUL terminator.
    var text: [i8: 14] = "Hello World!\n";
    text[13] = 0;
    val view = std::ConstStringView {@text[0], cast<u64>(13)};
    std::print!("{}", view);
    println!("numbers[1]={}", numbers[1]);
    return 0;
}
