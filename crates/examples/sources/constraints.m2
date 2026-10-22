// constraints: Combine named requirements with &&; all requirements must hold.
// Run: mlc run constraints.m2
// Expected output: answer=42
import std;


using println = std::println;
generic Integer {
    std::generic::is_integer;
};

generic Signed {
    std::generic::is_signed;
};

generic Wide {
    std::generic::min_bits < 32 >;
};

generic SignedInteger = Wide && Integer && Signed;

func<T: SignedInteger> twice(value: T) -> T {
    return value + value;
}


func main() -> i32 {
    val answer = twice(21);
    // twice(1.5) fails: f64 is not an integer.
    // twice(cast<u32>(21)) fails: u32 is not signed.
    // twice(cast<i16>(21)) fails: i16 has fewer than 32 bits.
    // Integer && Float would fail when declared if Float required is_float.
    println!("answer={}", answer);
    return 0;
}
