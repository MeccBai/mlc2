# AST optimizer

`mlc_core` analyzes every body and performs mandatory constant evaluation for
`const`, case labels, range checks, and type checking. It retains non-const
initializer expressions for optional optimization.

`mlc_optimizer` accepts analyzed ASTs and returns transformed ASTs. It depends
only on core, not LLVM, codegen, or the builder. The package symbol table is
passed by mutable reference because globals and generic instance bodies live
there. Symbol and type indices remain unchanged.

```rust
let optimizer = mlc_optimizer::Optimizer::new(mlc_optimizer::OptimizeConfig {
    constant_folding: true,
    dead_code_elimination: true,
    unused_variable_elimination: false,
    unused_function_elimination: false,
});
let asts = optimizer.optimize_all(analyzed_asts, &mut package);
```

Use `optimize` for a standalone translation unit; use `optimize_all` for package
builds so cross-file calls and function addresses are considered together.
Additional body passes can implement `AstPass` and be added with `add_pass`.

Pass order is constant folding, constant control-flow elimination, unused local
storage elimination, custom body passes, then package-wide zero-reference
function elimination. Selected branches remain anonymous scopes for cleanup.
Unused resource owners and aggregate initializers are conservatively retained;
other unused initializers retain their observable effects. No loop unrolling,
mutable-local propagation, or whole-program reachability analysis is performed.

Defaults preserve the earlier optimizations: folding and control-flow pruning
on, unused variable/function elimination off. `enabled()` and `disabled()` set
all four switches. The CLI exposes `--optimize` and `--no-optimize`; settings
participate in the object cache key. These switches do not control LLVM backend
optimization or generate debug information.
