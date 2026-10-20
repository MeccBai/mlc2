# Native str

`str` is the native immutable compile-time UTF-8 string type. It requires no
import. String literals infer `str`, and `str` has no explicit length parameter.

```text
val greeting:str="hello";
val inferred="hello";
const bytes:u64=str::len("中\0文"); // 7 UTF-8 bytes
val pointer:const $i8=const_c_str(greeting); // Explicit C interoperation
```

A local or global `str` binding must have a compile-time initializer and is
constant even when introduced with `var`. Reassignment and mutable borrowing
are errors. Runtime function calls are not compile-time evaluators; their
results cannot initialize a `str` binding. Bytes, raw pointers, and runtime
String views cannot be converted to `str`.

Functions can accept `str` parameters and inspect their byte length or explicitly
obtain a read-only C pointer. The implementation carries a pointer to static,
NUL-terminated UTF-8 storage plus its byte length. The trailing terminator is
excluded from the length, and embedded NUL bytes are preserved. `str` does not
implicitly convert to `const $i8`; call `const_c_str` explicitly.

Formatting macros take `pattern:str`. Macro parameters typed as `str` require
compile-time string arguments. The Display implementation formats a `str`
without losing embedded NUL bytes.

Explicitly declared byte arrays such as `var bytes:[i8:5]="hello";` retain their
existing copy and mutation behavior. They are distinct from inferred `str`.
