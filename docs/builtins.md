# Built-in functions

`cast`, `str::len`, `const_c_str` and `std::function` are compiler-owned names.
Memory operations require `import std::mem;`; the old bare `alloc`/`dealloc`
interfaces are not provided. Imported memory declarations lower to compiler intrinsics.

```rust
import std::mem;
func main() -> i32 {
    var p:res = std::mem::alloc<i32>(10);
    p[0] = 42;
    var small:i8 = cast<i8>(300); // 44: explicit narrowing keeps low bits
    std::mem::dealloc<i32>(p); // consumes p; no subsequent use
    return 0;
}
```

- `std::mem::alloc<T>(count)` allocates storage for `count` elements of `T`, returning
  an owning `res $mut T`. It lowers to `malloc(sizeof(T) * count)`; memory is uninitialized.
  Constant negative counts are errors. Runtime negative counts, size multiplication
  overflow, and null allocation results terminate the program through `abort`.
  A zero-size allocation still follows the allocator's result and the null check.
- `std::mem::dealloc<T>(ptr)` consumes an owning resource whose base matches `T`,
  releases its allocation and invalidates the owner. Ordinary non-owning Ref values
  cannot be passed instead. Scope exit automatically cleans up resources not already
  moved or released. Units containing resources receive recursive cleanup.
- `cast<T>(input)` converts numeric values or explicitly reinterprets reference
  types. Numeric conversions do not check value ranges. Integer narrowing keeps
  the low bits; widening respects source signedness. Float-to-integer conversion
  truncates toward zero; values outside the representable range have LLVM poison
  semantics, not a defined wrapping result. Integer constant casts are folded.
- `str::len(text)` accepts native `str` and returns its UTF-8 byte length as `u64`.
  Literal lengths are folded during compilation. Embedded NUL bytes count toward
  the length; the trailing C terminator does not.
- `const_c_str(text)` explicitly accepts native `str` (including a `str` parameter),
  appends one NUL byte after its UTF-8 bytes, and returns an immutable `$i8`.
  Storage is a deduplicated LLVM global constant, not stack or heap memory.
  Use it only when a C API needs a NUL-terminated pointer; never deallocate it.
  For ordinary output, use `std::println!("{}", text)` directly instead of
  converting to a C string. Embedded NULs are retained in the C storage.

Ordinary initialization, assignment, and return conversions continue to reject
out-of-range integer constants. A later implicit narrowing remains checked:
`var x:i8 = cast<i32>(300);` is still invalid.

Type names are not conversion functions: `i8(300)` is unsupported. The only
builtin conversion interface is `cast<i8>(300)`.

`std::function(name)` obtains a function pointer; `std::function<T,...>(name)`
explicitly instantiates its generic target. The pointer type is `func(args) -> T`;
Interface addresses and capturing lambdas are not supported.

Resource ownership does not provide a complete borrow checker. Ordinary references
can be indexed, and callers remain responsible for bounds and alias lifetime safety.
Allocation failure is not a recoverable null result and does not unwind or run cleanup.
