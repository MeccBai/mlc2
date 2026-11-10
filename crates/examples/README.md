# MLC examples

The 26 files in `sources/` are independent programs, embedded in the compiler.
Each source explains its topic, run command, and expected output in English.

```powershell
mlc example list
mlc example hello-world > hello.m2
mlc run hello.m2
mlc example formatting > formatting.m2
mlc run formatting.m2
mlc example const-functions > const_functions.m2
mlc run const_functions.m2
```

Listing and printing source need no project, configuration, or LLVM initialization.
For repository development, use `mlc run hello.m2 --lib-dir lib`. Installed mlc
uses its sibling `lib` directory by default.

Ordinary examples use `std::println!` and `std::print!`. The standard-library
examples cover owned String storage, non-owning byte-length views, `format!`,
and custom Display implementations. The dedicated `c-interop` example covers
C ABI declarations, `printf`, and NUL-terminated strings.

Resource examples distinguish explicit deallocation from automatic cleanup after
ownership transfer. Floating-point narrowing uses an explicit `cast<f32>`.

Tests parse every source, verify CLI source output, and build and run every
example with exact expected stdout comparisons.

The `const-functions` example evaluates loops, recursion and nested calls in
`const` initializers. It also shows an immutable `val` binding initialized by a
runtime function call. Const functions can call const functions and compiler
intrinsics. Const functions can pass references to other const functions. Calls
from ordinary functions or global initializers cannot pass pointer parameters,
including pointers nested in arrays, units or unions. Non-const calls, C function calls and access to global
var or val bindings are errors. Global const bindings and local pointer operations
are allowed.
