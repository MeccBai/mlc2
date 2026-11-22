# 调试信息实现

Native CodeView reserves line number 0xf00f00 for NeverStepInto. LLVM filters that number when reading DILocation, so codegen uses the reserved placeholder 0xefffff and the COFF writer restores only those line-number fields inside validated C13 DEBUG_S_LINES records. Addresses, relocations and instruction bytes remain unchanged. LLDB NativePDB currently discards these special rows ([upstream reader](https://github.com/llvm/llvm-project/blob/main/lldb/source/Plugins/SymbolFile/NativePDB/SymbolFileNativePDB.cpp)). Debug builds therefore also route these calls through internal forwarding functions with no source locations, noinline, optnone and disabled tail calls. Under default LLDB source stepping, the opaque frame is stepped over as a whole; explicit argument calls execute before that frame and remain debuggable. Direct calls to the same target retain normal debug information. Release builds do not emit forwarding functions. Explicit breakpoints inside the target and instruction stepping remain available. Debug builds can show an extra forwarding frame when stopped inside the target.

Validation:

```powershell
cargo test --test debug_info -- --include-ignored
llvm-readobj --codeview build/objects/debug/x86_64-pc-windows-msvc/main.obj
llvm-pdbutil dump -summary -l build/debug/main.pdb
```

The runtime test requires the bundled native linker and CRT libraries. The diagnostic tools are only needed for manual inspection.
