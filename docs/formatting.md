# Formatting and console output

Import the standard library and optionally introduce local macro aliases:

```text
import std;
using format = std::format;
using print = std::print;
using println = std::println;

func main() {
    var message = format!("value = {}", 42);
    print!("value = {}", 42);
    println!("; ready = {}", true);
    println!();
}
```

All three macros accept a native `str` pattern followed by values satisfying
`std::fmt::Display`. Patterns support `{}` placeholders and `{{` / `}}` literal
braces. Placeholder counts are checked during compilation. Literal patterns and
local/global `str` bindings are supported. `str` is native and requires no
standard-library import; see [Native str](str.md). Width, precision, and named
placeholders are not implemented.

`format!` returns `std::String`. `print!` writes without a trailing newline;
`println!` appends a newline, and `println!()` writes an empty line. Both output
macros have no return value. Each argument is evaluated once; arguments follow the
ordinary function parameter ownership rules.

The private `std::fmt::FormatWriter` borrows the result buffer and appends literal
segments in batches. Finalization fills the existing buffer; it does not clone
an owned String. Each primitive conversion uses stack scratch space and creates
its returned String with a single allocation. Custom Display implementations
continue to use the same `format(self)->String` contract.

Signed and unsigned integer conversion is implemented in standard-library source,
including the minimum signed 64-bit value. Floating-point conversion uses the
Windows CRT `_ecvt_s` only for rounded significant digits; punctuation, trailing
zero removal, sign and scientific notation are handled by the standard library.
It preserves the previous 17-significant-digit general format, including f32
promotion to f64. Negative zero is `-0`; infinities and NaNs use `inf`, `-inf`,
`nan`, and `-nan`. There are no `snprintf` calls in the formatting library.

`print!` and `println!` assemble one result and write it through buffered CRT
`fwrite`, sharing stdout with C stdio. A newline is appended before the write.
Partial writes are retried until completion or until the stream makes no progress.
Explicit byte counts preserve embedded NUL bytes and percent signs. No per-byte
`putchar` calls or variadic format-string dispatch are used for these macros.
