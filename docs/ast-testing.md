# 源码到 AST 的端到端测试

入口：`../crates/core/src/ast/tests.rs`。所有用例从普通源码字符串开始，依次经过
lexer、Chumsky parser、`AbstractSyntaxTree::new` 和 `analysis(&mut package)`，
不手工组装 Temp AST 或符号表。new 只持有分组后的 Temp 数据，analysis 才进行语义分析。

生产 AST 不持有 SymbolTable：Config 持有 FileId，PackageSymbolTable 的 ArenaStore
按 FileId 管理每个文件的符号表。所有 ArenaIndex 同时携带文件身份和本地位置。
旧的单文件测试通过仅限测试的 AnalyzedAst 适配器访问完成分析后的符号表；
`tests/stages.rs` 直接测试生产入口、多文件路由及 export/analysis 顺序。

export 与 analysis 共用声明登记和签名解析阶段。export 返回索引组成的 ExportTable，
不构建普通函数 Body 或全局变量初始化；analysis 继续消费保留的 Body。
导出表的 searchable 存放公开声明，inner 保留私有声明及泛型模板的索引，
inner 查找要求调用者 FileId 与来源文件一致。实体和模板仍属于 package。
entry 持有唯一的 PackageSymbolTable，各翻译单元始终在其中登记 arena 和 Config。
`package.concat(export_table)` 接收普通导出表，不接收另一个 package；export 自动调用
此入口发布导出索引，冲突时 package 不改变。lookup 先查 searchable，再查调用文件的 inner。
所有翻译单元共用同一个 GlobalConfig（克隆共享计数），由它递增分配 FileId。
core 不负责读取 import 文件；builder 的 BuildPlan 递归发现源码和声明文件，
由顶层 driver 在同一个 package 中执行多文件 export/analysis 并登记导入范围。

项目已拆为 workspace。前端测试属于 mlc_core library 的内部 `#[cfg(test)]` 模块，
使用 `cargo test -p mlc_core` 独立运行，不依赖 LLVM；它们不是根目录 `tests/` 独立测试 crate。

```powershell
cargo test ast::tests
cargo test
```

## 检查口径

目录按职责划分：`ast/module/` 保存 Temp 分组、声明解析与 Body 分析，
`ast/symbols/` 保存符号表、package、导入和导出定义；`ast/tests/` 保存整体
源码到 AST 测试及其 support。组件测试位于组件自己的目录，多组测试集中在
该组件的 `tests/` 下，不与生产实现文件混放。

- 合法源码：三个阶段不 panic，AST 未被标记为有毒，无语义错误。
- 语义非法源码：词法和语法必须成功，AST 构建不 panic，提交且仅提交一个错误。
- 语法非法源码：必须由 parser 返回错误，不进入 AST 构建。
- 部分用例进一步检查错误类型与源码 span、变量三态、循环变量 Rc 关联、
  初始化目标类型、match 兜底节点和泛型实例缓存。
- 截断测试对五份合法源码的每个 UTF-8 边界截断，检查不完整输入不会触发 panic。

## 已确认的待完成项

`known_gaps.rs` 保存曾经缺失行为的回归测试，目前已全部启用。

```powershell
cargo test ast::tests::known_gaps
```

当前已修复并覆盖：

1. 枚举变体解析为携带枚举类型及成员序号的常量。
2. owner 限定的普通 Interface 调用。
3. owner 限定的泛型 Interface 调用。

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
