# AST source locations

Semantic Expression and CompAtom variants carry a source Span. Use span() to read it; with_span() retains a source range when constructing a replacement.

- Expression::new keeps the parser's range, including nested expressions and individual variable uses.
- Expression/CompAtom conversions preserve the same range.
- Constant folding and numeric normalization keep the replaced expression's range. Substituting a named constant keeps the use site, not its initializer's location.
- Source positions do not participate in expression or atom semantic equality.
- FuncSymbol, InterfaceSymbol, FuncBody and Interface retain definition ranges. Generic instances inherit their template's range in the defining file, identified by their arena index.
- Variable already has declaration_span. This is separate from the location of each reference to that variable. Range-loop bindings now retain their actual declaration span too.
- Compiler-created nodes without a source location may use 0..0. Artificial nodes derived from source expressions inherit the relevant source range.

These positions now feed the codegen DebugInfoBuilder and Windows CodeView/PDB line tables; see [basic debugging information](debug-info.md). Debug builds additionally retain statement boundaries as SourcePosition nodes. Individual parameter declaration positions and local-variable debug records remain future work.
