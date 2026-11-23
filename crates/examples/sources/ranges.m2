// ranges: Half-open ranges, stepping and borrowing each element.
// Run: mlc run ranges.m2
// Expected output: plain=6 stepped=9
import std;

func main() -> i32 {
    const zero: u64 = 0;
    const four: u64 = 4;
    const one: u64 = 1;
    const six: u64 = 6;
    const step: u64 = 2;
    var plain: u64 = 0;
    var stepped: u64 = 0;
    for value in std::range(zero, four) {
        plain = plain + value;
    }
    for $value in std::range_step(one, six, step) {
        stepped = stepped + $value;
    }
    std::println!("plain={} stepped={}", plain, stepped);
    return 0;
}
