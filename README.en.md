# MLC2

[中文](README.md) | **English**

MLC2 is a statically typed language compiler written in Rust, with an LLVM backend. It also provides project builds, incremental caching, formatting, and C declaration import tools.

The project began as a compiler experiment and undergraduate thesis, with embedded development as its intended direction. **Windows x64 / MSVC / UCRT** is the main tested environment today. The language, standard library, and toolchain are evolving; ABI stability and production compatibility are not promised.

## A taste of the language

```rust
import std;

func<T> swap(a: $mut T, b: $mut T) {
    var temporary = $a;
    $a = $b;
    $b = temporary;
}

func main() -> i32 {
    var a = 10;
    var b = 20;
    swap(@mut a, @mut b);
    std::println!("a={} b={}", a, b);
    return 0;
}
```

Generic arguments may be explicit or inferred from call arguments. `@` / `@mut` create read-only / mutable references; `$` dereferences them. An ordinary `main` does not need a C ABI attribute.

## Implemented features

- **Types and expressions:** postfix type annotations, type inference, arrays, named aggregates (`unit`), enums, tagged unions, and function pointers.
- **Generics and interfaces:** constraints, monomorphization, generic argument inference, and type-bound interfaces; compile-time macro functions with heterogeneous argument-pack expansion.
- **Resource management:** no GC; `res` ownership, moves, invalidation checks, and automatic recursive destruction. References are distinct from ownership; this is not a full Rust-style borrow checker.
- **Standard library and FFI:** `String`, `format!`, `print!`, `println!`, and C API declarations imported through `.sym` files. The standard library is still growing.
- **Code generation and optimization:** textual LLVM IR, native objects, executables, static and shared libraries; a separate AST pass pipeline including constant folding and dead-code elimination.
- **Developer tools:** dependency-driven builds, object and link caches, progress and timing summaries, source-context diagnostics, fixed-style formatting, and optional Windows debug information.

## Quick start

These commands assume a configured MLC installation on PATH, with its standard library, linker tools, and runtime libraries in place. See the [distribution guide](docs/distribution.md) for the layout.

Save the program above as `main.m2`:

```powershell
mlc run main.m2
# a=20 b=10

mlc build main.m2
# Default output: build/default/main.exe

mlc ll main.m2
# Emit LLVM IR only; no object compilation or linking

mlc fmt main.m2
```

`mlc build main.m2` uses a standalone entry file and ignores project configuration, but still resolves imports. Project builds use `Project.toml`:

```toml
[Project]
Name = "hello"
Version = "0.1.0"
OutputDir = "build"

[[Project.Targets]]
Name = "app"
Entry = "main.m2"
Type = "bin"
```

```powershell
mlc build          # Build every project target
mlc run app        # Build and run a named target
mlc fmt            # Format the project's .m2 sources
mlc example list   # List embedded examples
mlc example generics
```

Build modes are `--default`, `--debug`, `--release`, and `--rel-with-info`, with separate output directories. The latter two enable optimization; `debug` and `rel-with-info` enable debug information. These MLC modes are separate from the Cargo profiles used to build the compiler itself.

## Importing C declarations

External C functions are not declared without bodies in `.m2` files. Convert a restricted header to `.sym` first:

```c
// native.h — no #include is needed or allowed
struct Point {
    int32_t x;
    int32_t y;
};
int32_t abs(int32_t value);
void update(struct Point *point);
```

```powershell
mlc symbols native.h   # Produces native.sym; use import native;
```

The importer accepts explicit-width integer types, supported floating-point types, pointers, and restricted struct/function declarations. It rejects `int`, `long`, `char`, `size_t`, preprocessing, typedefs, inline functions, and extension attributes. Passing or returning C structs by value is not supported yet. MLC functions with bodies may still use `#[c_abi]#` for C-compatible exports.

See the [FFI parser](crates/ffi_parser/README.md) and [C API modules](docs/c_std.md) for details.

## Building from source

The current build script is configured for a Windows development environment. **Installing Rust alone is not enough for a portable build:**

1. A Rust toolchain supporting edition 2024.
2. LLVM 23.1 development libraries matching `llvm-sys 231`. The script currently links `$env:SCOOP/apps/llvm-dev/current/lib/LLVM-C.lib`; `SCOOP` must be set.
3. `mlc-build` requires `LLVM-C.dll` beside its executable or through the system DLL search path. The `mlc` router and `mlc-fmt` formatter do not load LLVM. Deploy all three executables in the same directory.
4. Windows linking requires `lld-link` and MSVC/UCRT/Windows SDK libraries; static archives use `llvm-ar`. Instrumented C tests also need `clang-cl`, used only to compile fixtures, not as the linker driver.

```powershell
cargo build --release --bins
./target/release/mlc.exe --help

# Frontend, header parser, and formatter tests
cargo test -p mlc_syntax -p mlc_core -p ffi_parser -p mlc_formatter

# Includes LLVM object generation and native execution; requires the toolchain above
cargo test --workspace
```

For development, point MLC at the repository root and prepare the registered standard packages:

```powershell
$env:MLC = (Get-Location).Path
./target/release/mlc.exe setup
./target/release/mlc.exe run crates/examples/sources/hello_world.m2
```

## Documentation

- [Language usage](docs/README.md#语言使用)
- [Compiler implementation](docs/README.md#实现)

Features and syntax may change. When reporting an issue, include a minimal `.m2` / `.h` reproducer, the command, target triplet, and full diagnostic output.
