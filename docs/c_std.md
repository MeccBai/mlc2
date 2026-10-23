# C standard library declarations

These modules provide raw C ABI declarations. Header inputs are converted to declaration-only .sym files during installation; import does not parse C headers.

| Module | Functions | Location |
| --- | --- | --- |
| c_std::lib | Numeric parsing, allocation, abs, abort/exit, random numbers and environment operations | universal/lib.h |
| c_std::string | Memory copying/comparison, string length, copying, search, tokenization and error messages | universal/string.h |
| c_std::io | Console and file I/O, buffering, formatted input/output, temporary files and filesystem names | universal/io.h |
| c_std::math | Double and float trigonometry, powers, logarithms, rounding and related operations | universal/math.h |
| c_std::ctype | Character classification and case conversion | universal/ctype.h |
| c_std::locale | setlocale | universal/locale.h |
| c_std::lib_platform | Windows long-width atol/labs/strtol/strtoul | each Windows triplet |
| c_std::io_platform | Windows long/fpos_t offsets and explicit 64-bit seeking | each Windows triplet |
| c_std::time | Windows tm, clock and explicit UCRT 64-bit time APIs | each Windows triplet |
| c_std::runtime | Existing abort, abs, fwrite, snprintf, strcmp and UCRT helpers | each Windows triplet (unchanged) |

Modules follow their role in c_std rather than C header filenames: use c_std::io (not stdio), c_std::lib (not stdlib), c_std::string::strcmp and c_std::io::fwrite. The old io.toml has been replaced by io.h. runtime retains the existing declarations for compatibility with standard-library internals.

## Example

```text
import c_std::lib;
import c_std::string;
import c_std::math;
import c_std::io;

func main()->i32 {
    var n=c_std::lib::atoi(const_c_str("42"));
    var length=c_std::string::strlen(const_c_str("hello"));
    c_std::io::printf(const_c_str("n=%d length=%llu sqrt=%.1f\n"),n,length,c_std::math::sqrt(81.0));
    return 0;
}
```

## ABI and ownership

These headers describe the compiler's currently supported 64-bit ABIs: C int is i32, size_t is u64, and long long is i64. Size arguments generally need explicit cast<u64>(...) in MLC. Character buffers use i8 pointers; const_c_str supplies NUL-terminated input. Returned pointers are raw C pointers, without automatic ownership or lifetime tracking. Only free memory allocated by the matching C allocator; do not free strings returned by getenv, strerror or setlocale.

FILE is an opaque void pointer. Open it with fopen/tmpfile and close it with fclose. C stdout/stderr globals and FILE internals are not exposed. Windows tm is a complete nine-i32 struct. Its tm_year counts years since 1900 and tm_mon starts at zero.

Functions involving C long, fpos_t, time_t or tm layout are kept out of universal even when the function name is standardized. Windows time uses explicit _time64/_gmtime64_s-style exported names, avoiding header-only aliases. Linux/macOS target-specific versions are not supplied by this Windows bundle.

Function-pointer APIs (qsort/bsearch/atexit/signal), va_list APIs, long double, wide-character and macro-only interfaces are omitted because the restricted parser cannot faithfully describe them. UCRT does not export callable fabsf/frexpf/hypotf/ldexpf symbols, so those four names are omitted; the corresponding double APIs are available. C macro constants and globals are not synthesized.

## Validation and installation

```powershell
mlc symbols lib/universal/c_std/string.h --module c_std::string --triplet universal -o objects/universal/c_std/string.sym
mlc symbols lib/x86_64-pc-windows-msvc/c_std/time.h --module c_std::time --triplet x86_64-pc-windows-msvc -o objects/x86_64-pc-windows-msvc/c_std/time.sym
cargo test --test c_std --test ffi_symbols_cli
```

Tests verify universal and target metadata, resolve every declared Windows function against UCRT, and run representative functions through both MSVC and GNU linking. Universal declarations also compile for supported Linux/macOS targets; runtime availability there still depends on the platform C library and linker setup (for example libm).
