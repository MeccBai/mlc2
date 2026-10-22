# Basic debugging information

Use `mlc build main.m2 --debug` or `mlc run main.m2 --debug`. `--release` disables debug information. The flags are mutually exclusive; no separate -g option is exposed.

Project builds may set `Mode = "debug"` or `Mode = "release"` in [Project] or an individual [[Project.Targets]] table. Precedence: command line, target, project, release default. A standalone .m2 entry continues to ignore Project.toml.

Debug builds disable optional AST and LLVM machine-code optimizations. CodegenConfig enables a codegen-owned DebugInfoBuilder. LLVM receives line-table metadata; Windows COFF objects carry CodeView, and the native linker emits a sibling PDB. Debug settings, generated IR and PDB contents participate in cache validation; a deleted or changed PDB forces relinking.

The initial scope is source files, ordinary function definitions, statements, expressions and call sites. Local variable/type inspection is not implemented yet.

- Ordinary functions have source line tables and can be stepped into.
- Each generic template has one shared declaration node and source location. Each machine-code instance has its own definition and line table referencing that declaration, so all instances can be stepped into at the template source.
- Macro helper bodies have no debug declarations or expansion line tables. Macro calls are marked NeverStepInto; explicit argument calls retain their own source locations.
- Pipeline stage calls are marked NeverStepInto. Explicit function calls inside their arguments keep normal locations, so stepping into those arguments is possible.
- Generated entry, initialization and destruction helpers have no user line tables.

Native CodeView reserves line number 0xf00f00 for NeverStepInto. LLVM filters that number when reading DILocation, so codegen uses the reserved placeholder 0xefffff and the COFF writer restores only those line-number fields inside validated C13 DEBUG_S_LINES records. Addresses, relocations and instruction bytes remain unchanged. LLDB NativePDB currently discards these special rows ([upstream reader](https://github.com/llvm/llvm-project/blob/main/lldb/source/Plugins/SymbolFile/NativePDB/SymbolFileNativePDB.cpp)). Debug builds therefore also route these calls through internal forwarding functions with no source locations, noinline, optnone and disabled tail calls. Under default LLDB source stepping, the opaque frame is stepped over as a whole; explicit argument calls execute before that frame and remain debuggable. Direct calls to the same target retain normal debug information. Release builds do not emit forwarding functions. Explicit breakpoints inside the target and instruction stepping remain available. Debug builds can show an extra forwarding frame when stopped inside the target.

Validation:

```powershell
cargo test --test debug_info -- --include-ignored
llvm-readobj --codeview build/objects/x86_64-pc-windows-msvc/main.obj
llvm-pdbutil dump -summary -l build/main.pdb
```

The runtime test requires the bundled native linker and CRT libraries. The diagnostic tools are only needed for manual inspection.
