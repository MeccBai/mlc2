# Basic debugging information

Build modes are mutually exclusive; no separate -g option is exposed.

| CLI | Project.toml Mode | Optimization | Debug information |
| --- | --- | --- | --- |
| no flag / `--default` | `default` | off | off |
| `--debug` | `debug` | off | on |
| `--release` | `release` | on | off |
| `--rel-with-info` | `rel-with-info` | on | on |

For example, `mlc run main.m2 --rel-with-info`. Existing `--optimize` and `--no-optimize` explicitly override only the optimization setting, without changing debug information.

Project builds may set any of the four `Mode` values above in [Project] or an individual [[Project.Targets]] table. Precedence: command line, target, project, unoptimized default. A standalone .m2 entry continues to ignore Project.toml.

Windows builds with debug information produce a PDB beside the executable. In optimized builds, stepping may skip or combine source statements.

The initial scope is source files, ordinary function definitions, statements, expressions and call sites. Local variable/type inspection is not implemented yet.

- Ordinary functions have source line tables and can be stepped into.
- Generic functions can be stepped into at the template source.
- Stepping normally skips macro bodies. Explicit function calls in arguments remain debuggable.
- Stepping normally skips pipeline stages. Explicit function calls in their arguments remain debuggable.
- Generated entry, initialization and destruction helpers have no user line tables.
