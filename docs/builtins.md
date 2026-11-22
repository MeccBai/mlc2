# Built-in functions

`cast`, `str::len`, `const_c_str` and `std::function` are compiler-owned names.
Memory operations require `import std::mem;` or an import that re-exports it; the old bare `alloc`/`dealloc`
interfaces are not provided.

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

- `std::mem::address<T>(address)` creates a mutable reference at an integer address.
  Use `u32` on 32-bit targets and `u64` on 64-bit targets.
- `std::mem::alloc<T>(count)` allocates storage for `count` elements of `T`, returning
  an owning `res $mut T`. The allocated memory is uninitialized.
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
  truncates toward zero; the truncated result must fit the destination type.
- `str::len(text)` accepts native `str` and returns its UTF-8 byte length as `u64`.
  Literal lengths are folded during compilation. Embedded NUL bytes count toward
  the length; the trailing C terminator does not.
- `text.first_byte()` returns the first raw UTF-8 byte as `u8`, or zero for an empty string.
- `str::as_ptr(text)` returns a read-only pointer to the UTF-8 bytes.
- `str::from_parts(ptr, len)` creates a view over an existing UTF-8 byte range.
- `const_c_str(text)` returns a read-only pointer for C APIs that expect
  NUL-terminated input. String literals already include a trailing NUL.
  See [Native str](str.md) for pointer lifetime and raw view requirements.

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

## Numeric constant intrinsics

Integer types expose `min()` and `max()` for their exact limits. `f32` and `f64` expose `min()` and `max()` for their most negative and largest finite values, `min_positive()` for the smallest positive normal value, and `epsilon()` for the spacing above 1. The same intrinsics are also available with the `std::` prefix.

```text
const upper:u32 = u32::max();
const lower:i32 = i32::min();
const tolerance:f64 = f64::epsilon();
```

These compiler-owned, zero-argument functions return typed compile-time constants, including in default/debug builds. They do not require importing a library module, accept no generic arguments and do not enable implicit narrowing or signedness conversion. Call them directly; they are not runtime function-pointer targets.

## todo()

内建 todo() 接受零个参数，可填入有预期类型的表达式位置：

~~~m2
unit Item { pub value: i32; };

func load() -> Item {
    return todo();
}

func main() -> i32 {
    var result: f64 = todo();
    return 0;
}
~~~

它不产生真实的替代值。正常编译后，一旦执行到该位置，就打印当前源文件、行、列
和 not implemented (todo)，然后调用退出接口以 -1 结束进程。无需 import std。
退出码在不同操作系统上可能表现为不同的非零整数。编辑器分析只检查它，不执行它。
