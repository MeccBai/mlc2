# Unified semantic inference

`mlc_core::TypeInfer` owns the inference session: diagnostic `Config`, symbol/
type storage through `Resolution`, and an optional generic-name `TypeContext`.
It does not own the AST, frames, resource-move state, or LLVM ABI mappings.

```rust
let mut infer = mlc_core::TypeInfer::new(&mut config, &mut symbols);
let result = infer.infer(&expression);
let accepted = infer.check(
    &expression,
    expected_type,
    mlc_core::infer::CheckMode::Initialization,
    span,
);
```

## Responsibilities

| Entry | Responsibility |
| --- | --- |
| `infer` | Return `Value(Inferred)`, `Void`, `NeedsExpectedType`, or `Poison` |
| `type_of` | Low-level type query, retaining the existing `TypeIndex::empty()` convention |
| `assignable` | Determine writable-place permissions, including Ref base mutability |
| `check` | Context-sensitive checking and diagnostic submission at the supplied Span |
| `check_arguments` | Fixed/variadic arity and shared argument checks for functions, interfaces and function pointers |
| `constant` / `is_const` | Conservative constant evaluation and constant-expression classification |
| `check_constant_range` | Integer bounds and numeric precision diagnostics |
| `normalize` | Normalize already accepted literal/aggregate initializers to their target types |
| `generic_arguments` | Infer a fresh generic binding map from a function signature and arguments |
| `with_context(...).resolve_type(...)` | Resolve annotations with unit/function/statement generic bindings |

`Inferred` contains the type and `ValueCategory::{Value, Place { mutable }}`.
Ownership and Flex/Final/Constant remain properties of existing types; they are
not duplicated as new inference flags. A void function call is distinct from a
target-typed initializer that needs an expected type, and both are distinct from
poisoned input.

Initialization retains the existing value-aware numeric rules. Function
arguments remain strict. Assignment and return checks retain their existing
compatibility policy. Explicit `cast` still uses its builtin conversion rules;
the session does not introduce new implicit conversions.

Diagnostics continue through `Config::submit_error`; poisoned sessions stop
without producing follow-on mismatch diagnostics. Pass Span at the operation
that can report an error, rather than storing a mutable "current span" in the
session.

Read-only clients such as constant-folding passes can use
`TypeInfer::read_only(&mut config, &symbols)`. Such a session cannot intern
derived types or run expression/generic type inference. It can evaluate,
classify, range-check, and normalize constants without obtaining mutable arena
access.

Each `generic_arguments` call creates its own binding map. Recursive structural
matching covers generic variables, references/resources, arrays, function
signatures, and instantiated Unit/Union applications. Existing constraint
validation and monomorphization remain in the instantiation layer.

## Layout and migration

Implementation lives in `crates/core/src/infer/`, split by responsibility:
expression, place, check, constants, constantness, numeric, range, generics, and
resolve. Existing AST convenience methods delegate here instead of containing
separate algorithms. Structural leaf-type compatibility remains on the types.
No additional persistent AST representation or global inference cache is added.

Core's mandatory constant evaluation for `const`, range checking and annotations
still runs independently of optimizer configuration. Optional AST folding in
`mlc_optimizer` reaches the same implementation through the convenience API.
