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
3. `LLVM-C.dll` must be available beside the compiler executable or through the system's DLL search path.
4. Windows linking requires `lld-link` and MSVC/UCRT/Windows SDK libraries; static archives use `llvm-ar`. Instrumented C tests also need `clang-cl`, used only to compile fixtures, not as the linker driver.

```powershell
cargo build --release
./target/release/mlc.exe --help

# Frontend, header parser, and formatter tests
cargo test -p mlc_syntax -p mlc_core -p ffi_parser -p mlc_formatter

# Includes LLVM object generation and native execution; requires the toolchain above
cargo test --workspace
```

When running from a Cargo output directory, the compiler does not automatically locate the repository's `lib`. Prepare declaration symbols from the repository root, then use `--lib-dir lib`:

```powershell
$mlc = (Resolve-Path ./target/release/mlc.exe).Path
$library = (Resolve-Path ./lib).Path
Get-ChildItem $library -Recurse -Filter *.h | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($library, $_.FullName)
    $parts = $relative -split '[/\\]'
    $module = ($parts[1..($parts.Length - 1)] -join '::') -replace '\.h$', ''
    $output = Join-Path objects ([IO.Path]::ChangeExtension($relative, '.sym'))
    & $mlc symbols $_.FullName --module $module --triplet $parts[0] -o $output
    if ($LASTEXITCODE -ne 0) { throw "Header conversion failed: $relative" }
}
& $mlc symbols lib/universal/std/mem.toml -o objects/universal/std/mem.sym
if ($LASTEXITCODE -ne 0) { throw 'std::mem conversion failed' }
& $mlc ll main.m2 --lib-dir lib
```

`cargo build` alone does not create a complete installation or download CRT libraries and linker tools. Running/linking programs also requires configured `tools` and `runtimes`; a development setup can explicitly use a linker on PATH with `--linker lld-link`. See the [distribution guide](docs/distribution.md) for tool placement, library paths, and packaging.

Windows defaults to `x86_64-pc-windows-msvc`; GNU targets remain an explicit option. LLVM object generation for another target does not imply a complete standard library, linker profile, or runtime for it. Embedded toolchain integration remains future work.

## Pipeline and workspace

```text
.m2 → Lexer / Chumsky → Temp AST → Type and semantic analysis → AST passes
                                                                  ↓
                              Native objects / linking ← LLVM ← Text IR
```

| Crate | Responsibility |
| --- | --- |
| `mlc_syntax` | Lexer, Chumsky parser, Temp AST, spans, and source line tables |
| `mlc_core` | Symbols, types, inference, semantic checks, generic instantiation, resource semantics |
| `mlc_optimizer` | Optimization passes over analyzed ASTs |
| `mlc_codegen` | LLVM IR generation, ABI adaptation, destruction code |
| `mlc_builder` | Dependency scheduling, caches, `.sym` / `.mg` artifacts, LLVM object emission |
| `mlc_formatter` | Comment-preserving, fixed-style formatting using the syntax layer |
| `ffi_parser` | Restricted, explicit-width C declaration parsing |
| `mlc_examples` | Embedded language examples |
| `mlc` | CLI, cross-module orchestration, final linking |

## Documentation

Most detailed documentation is currently in Chinese.

- [Language syntax and examples](docs/language.md)
- [CLI and build modes](docs/cli.md)
- [Example guide](docs/examples.md)
- [C API declaration modules](docs/c_std.md)
- [Build pipeline and caching](docs/build_pipeline.md)
- [Installation layout and distribution](docs/distribution.md)
- [Formatting rules](docs/formatting.md)
- [Debug information](docs/debug-info.md)
- [Workspace design](docs/workspace.md)

Features and syntax may change. When reporting an issue, include a minimal `.m2` / `.h` reproducer, the command, target triplet, and full diagnostic output.
