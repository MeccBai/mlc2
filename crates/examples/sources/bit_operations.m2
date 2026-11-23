// bit-operations: Import bit operations and use numeric type limits.
// Run: mlc run bit_operations.m2
// Expected output: flags=9 low=1 max=255
import std;
import std::bit;

func main() -> i32 {
    const bit: u8 = 1;
    const shift: u8 = 3;
    const flags = std::bit::or(bit, std::bit::shl(bit, shift));
    std::println!("flags={} low={} max={}", flags, std::bit::and(flags, bit), u8::max());
    return 0;
}
