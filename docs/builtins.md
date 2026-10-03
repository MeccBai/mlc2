# Built-in functions

`cast` and `const_c_str` are compiler-owned names and require no import.
Memory operations require `import std::mem;`; the old bare `alloc`/`dealloc`
interfaces are not provided. Imported memory declarations lower to compiler intrinsics.

```rust
import std::mem;
var p = std::mem::alloc<i32>(10);
$p = 42;
var small:i8 = cast<i8>(300); // 44: explicit integer narrowing keeps low bits
std::mem::dealloc<i32>(p);
var message = const_c_str("Hello World!\n"); // immutable $i8, static lifetime
```

- `std::mem::alloc<T>(count)` allocates storage for `count` elements of `T`, returning
  `$mut T`. It lowers to `malloc(sizeof(T) * count)`; memory is uninitialized.
  Constant negative counts are errors. Runtime negative counts, size multiplication
  overflow, and allocator failure produce null. Zero counts follow `malloc(0)`.
- `std::mem::dealloc<T>(ptr)` accepts a single-level reference whose base matches `T` and
  lowers to `free`. It does not run destructors. Only free null or the original
  allocation pointer, exactly once; aliases are not automatically invalidated.
- `cast<T>(input)` converts numeric values or explicitly reinterprets reference
  types. Numeric conversions do not check value ranges. Integer narrowing keeps
  the low bits; widening respects source signedness. Float-to-integer conversion
  truncates toward zero; values outside the representable range have LLVM poison
  semantics, not a defined wrapping result. Integer constant casts are folded.
- `const_c_str(text)` accepts a constant string (literal or folded constant),
  appends one NUL byte after its UTF-8 bytes, and returns an immutable `$i8`.
  Storage is a deduplicated LLVM global constant, not stack or heap memory.
  It can be passed directly to `c_std::io::printf`; never deallocate it or pass
  arbitrary user-controlled text as a printf format. Embedded NULs are retained.

Ordinary initialization, assignment, and return conversions continue to reject
out-of-range integer constants. A later implicit narrowing remains checked:
`var x:i8 = cast<i32>(300);` is still invalid.

Type names are not conversion functions: `i8(300)` is unsupported. The only
builtin conversion interface is `cast<i8>(300)`.

No automatic cleanup or allocation tracking is provided. Allocation references
may be null; callers must check before dereferencing. Direct indexing of array
references is still unsupported; this builtin change does not alter that rule.
