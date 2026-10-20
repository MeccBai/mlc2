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

The private `std::fmt::FormatWriter` implements pattern processing in standard
library source. Numeric conversions currently use C `snprintf`; console output
uses C `putchar` on the resulting bytes, preserving embedded NUL bytes and
avoiding interpretation of percent signs as format directives.
