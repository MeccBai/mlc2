### Comment

| Token   | SymbolName        | Token     | SymbolName       |
|---------|-------------------|-----------|------------------|
| `//...` | SingleLineComment | `/*...*/` | MultiLineComment |

### Literal

| Token | SymbolName   | Token | SymbolName | Token   | SymbolName    |
|-------|--------------|-------|------------|---------|---------------|
| `1.5` | FloatLiteral | `123` | IntLiteral | `"abc"` | StringLiteral |

### Operators

| Token | SymbolName  | Token | SymbolName | Token  | SymbolName     |
|-------|-------------|-------|------------|--------|----------------|
| `=`   | Assign      | `@`   | AddressOf  | `$`    | Dereference    |
| `->`  | Arrow       | `.`   | Dot        | `,`    | Comma          |
| `:`   | Colon       | `::`  | ColonColon | `+`    | Plus           |
| `-`   | Minus       | `*`   | Multi      | `/`    | Div            |
| `%`   | Mod         | `&`   | BitAnd     | `\|`   | BitOr          |
| `^`   | BitXor      | `~`   | BitNot     | `<<`   | Shl            |
| `>>`  | Shr         | `==`  | Equal      | `!=`   | NotEqual       |
| `>`   | RAngle      | `<`   | LAngle     | `>=`   | GreaterOrEqual |
| `<=`  | LessOrEqual | `&&`  | LogicalAnd | `\|\|` | LogicalOr      |
| `!`   | LogicalNot  |       |            |        |                |

### Other

| Token | SymbolName     | Token | SymbolName   | Token | SymbolName   |
|-------|----------------|-------|--------------|-------|--------------|
| `[[`  | AttributeStart | `]]`  | AttributeEnd | `\|>` | Pipe         |
| `=>`  | FatArrow       | `_`   | Default      | `(`   | LParen       |
| `)`   | RParen         | `{`   | LBrace       | `}`   | RBrace       |
| `;`   | Semicolon      | `[`   | LeftBracket  | `]`   | RightBracket |

### Keywords

| Token       | SymbolName | Token    | SymbolName | Token      | SymbolName |
|-------------|------------|----------|------------|------------|------------|
| `func`      | Function   | `var`    | Variable   | `const`    | Constant   |
| `unit`      | Unit       | `using`  | Using      | `generic`  | Generic    |
| `enum`      | Enum       | `import` | Import     | `export`   | Export     |
| `pub`       | Public     | `global` | Global     | `mut`      | Mut        |
| `in`        | In         | `true`   | True       | `false`    | False      |
| `if`        | If         | `else`   | Else       | `while`    | While      |
| `for`       | For        | `break`  | Break      | `continue` | Continue   |
| `match`     | Match      | `return` | Return     | `null`     | Null       |
| `anonymous` | Anonymous  | `...`    | VarList    |            |            |

### Identifier

| Token  | SymbolName |
|--------|------------|
| `name` | Ident      |
