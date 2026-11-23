// native-str: Read-only and rebindable strings, runtime values and UTF-8 bytes.
// Run: mlc run native_str.m2
// Expected output (/ separates lines): ascii=43 utf8=228 / text=中 bytes=3
import std;

func greeting() -> str { return "中"; }

func main() -> i32 {
    const plus: u8 = "+".first_byte();
    var text: mut str = "hello";
    text = greeting();
    std::println!("ascii={} utf8={}", plus, text.first_byte());
    std::println!("text={} bytes={}", text, str::len(text));
    return 0;
}
