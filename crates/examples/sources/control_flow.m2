// control_flow: for includes its start and excludes its end; _ is the default match branch.
// Run: mlc run control_flow.m2
// Expected output (/ separates lines): sum=10 / matched ten
import std;

using println = std::println;

func main() -> i32 {
    var sum = 0;
    // Numeric loop: 0 inclusive, 5 exclusive.
    for i in std::range(cast<u64>(0), cast<u64>(5)) {
        sum = sum + cast<i32>(i);
    }
    if (sum == 10) {
        println!("sum={}", sum);
    } else {
        return 1;
    }
    match (sum) {
        10 => {
            println!("matched ten");
        },
        _ => {
            return 2;
        }
    }
    return 0;
}
