# 源码到 AST 的端到端测试

入口：`../src/ast/tests.rs`。所有用例从普通源码字符串开始，依次经过
lexer、Chumsky parser 和 `AbstractSyntaxTree::new`，不手工组装 Temp AST 或符号表。

项目目前只有 binary crate，因此测试作为内部 `#[cfg(test)]` 模块运行；它们不是
Cargo 的根目录 `tests/` 独立测试 crate。将来拆出 library 后可以迁移。

```powershell
cargo test ast::tests
cargo test
```

## 检查口径

- 合法源码：三个阶段不 panic，AST 未被标记为有毒，无语义错误。
- 语义非法源码：词法和语法必须成功，AST 构建不 panic，提交且仅提交一个错误。
- 语法非法源码：必须由 parser 返回错误，不进入 AST 构建。
- 部分用例进一步检查错误类型与源码 span、变量三态、循环变量 Rc 关联、
  初始化目标类型、match 兜底节点和泛型实例缓存。
- 截断测试对五份合法源码的每个 UTF-8 边界截断，检查不完整输入不会触发 panic。

## 已确认的待完成项

`known_gaps.rs` 保存期望行为的回归测试，使用带原因的 `#[ignore]` 标记。
忽略不是将错误行为当成成功；修复后应去掉对应标记。

```powershell
cargo test ast::tests::known_gaps -- --ignored
```

当前这组测试会失败，已逐项运行确认：

1. 枚举变体值被当成变量路径，提交 UnknownVariable。
2. owner 限定的普通 Interface 调用找不到符号。
3. owner 限定的泛型 Interface 调用找不到符号。

赋值已改为左右表达式；测试覆盖变量、成员、下标和解引用左值及泛型实例化。
return 的值类型/有无值检查和 break/continue 的祖先循环检查已启用对应测试。
这里的 return 检查不包含“所有执行路径是否返回”的控制流分析。

Function 的参数数量与类型校验已启用，包括普通调用、管道和泛型调用。
类型使用与 Interface 相同的严格检查；末尾 `...` 允许任意数量的额外参数，
固定参数部分仍检查数量和类型。

`Box<T>` 的延迟实例化已修复。符号化 Unit 保留模板和参数，具体化后复用
规范的 Unit 实例。测试覆盖函数参数/返回类型、Body 局部变量、引用参数、
嵌套与递归 Unit、空 Unit 和约束检查。

函数/接口实例化现在按实例名复用缓存，替换签名后先登记新 symbol，再调用
Body 的实例化入口。Body 用新 symbol 的参数、返回类型及 owner 创建上下文，
在同一份泛型绑定下逐语句构建；不再保存备用真 AST 模板，也不再执行构建后的
整棵 Body 替换。测试检查原模板不变、不同实例的参数变量不共享，以及 Body
出错时清理 Actives。

Attribute 使用 `#[...]#`，不再与相邻的数组括号冲突；嵌套数组可以直接写成
`[[1,2],[3,4]]`。
