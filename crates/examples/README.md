# MLC examples

Each file in `sources/` is an independent program using implemented syntax.
The crate embeds these sources in mlc, so no separate example installation is needed.

```powershell
mlc example list
mlc example hello-world
mlc example generics
```

These commands only print text. Save a source as `main.m2`, then build it:

```powershell
mlc main.m2 --lib-dir lib
```

The explicit `--lib-dir lib` is for repository development. Installed mlc uses its
sibling `lib` directory by default. Printing needs neither LLVM nor a project config.
The library currently provides C printf declarations and compiler-backed memory
operations, not a complete standard library. Memory examples require manual cleanup.
