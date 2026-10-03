// var: mutable; val: immutable; const: compile-time constant.
import c_std::io;

func main() -> i32 {
    const base:i32 = 6 * 7;
    val answer = base;
    var counter = answer;
    counter = counter + 1;

    // Explicit cast permits truncation; implicit i8 = 300 is an error.
    val narrowed:i8 = cast<i8>(300);
    c_std::io::printf(const_c_str("answer=%d counter=%d narrowed=%d\n"),
        answer, counter, cast<i32>(narrowed));
    return 0;
}
