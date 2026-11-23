// iterator-constraints: Use the same element type in a bound and its function signature.
// Run: mlc run iterator_constraints.m2
// Expected output: first=7
import std;

func<T, I: std::collection::iter::Iterator<T>> first(iter: I) -> T {
    return iter.get_clone();
}

func main() -> i32 {
    const start: u64 = 7;
    const end: u64 = 10;
    var iter = std::range(start, end);
    std::println!("first={}", first<u64, std::RangeIter>(iter));
    return 0;
}
