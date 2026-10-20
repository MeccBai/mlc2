# 内置示例指南

目前共 25 个示例，从简单到复杂排列。每份都是独立的 `.m2` 程序，不需要 Project.toml。

```powershell
mlc example list
mlc example associated-generics > swap.m2
mlc run swap.m2
```

保存后的文件名可以任意指定；源码注释中的运行命令使用示例的原始文件名。
每份源码头部列出学习要点和预期输出，正常运行的退出码均为 0。
`mlc example <名称>` 只打印源码，不执行程序。

| 阶段 | 名称 | 学习内容 |
|---|---|---|
| 基础 | hello-world | main、import、std::println! |
| 基础 | variables | 可变变量、不可变绑定、常量与数值转换 |
| 基础 | arrays | 固定数组、索引、按字节长度构造字符串视图 |
| 基础 | control-flow | for 的半开区间、条件分支、match |
| 基础 | while-loops | while、break、continue |
| 基础 | references | @、@mut、$ 与引用可变性 |
| 函数与泛型 | functions | 参数、返回值、递归 |
| 函数与泛型 | aliases | 类型别名与函数别名 |
| 函数与泛型 | pipelines | 单值传入、参数拆分与连续调用 |
| 函数与泛型 | function-pointers | std::function 与回调 |
| 函数与泛型 | generics | 普通泛型函数的推断与显式参数 |
| 函数与泛型 | associated-generics | Base::swap(...) 与 Base::swap<f64>(...) |
| 函数与泛型 | constraints | && 合并类型、符号与位宽约束 |
| 类型与资源 | units | 字段顺序初始化与 pub |
| 类型与资源 | generic-units | Box<T> 构造与结构化泛型推断 |
| 类型与资源 | methods | self、mut self、点号与箭头 |
| 类型与资源 | enums | 枚举类型与值匹配 |
| 类型与资源 | unions | 带数据的候选类型与解包 |
| 类型与资源 | globals | main 前的运行期初始化 |
| 类型与资源 | memory | 分配、初始化、显式释放 |
| 类型与资源 | resources | 借用作用域、move 与自动释放 |
| 标准库 | strings | std::String、append、clone、非持有字符串视图 |
| 标准库 | formatting | format!、print!、println!、转义花括号 |
| 标准库 | display | 自定义 format 接口与 Display 约束 |
| 互操作 | c-interop | C ABI、printf、NUL 终止字符串 |

注意 `unit<T> Box` 与 `Base::func<T>` 的泛型归属不同：前者参数属于类型，
后者参数属于函数。当前关联泛型示例使用普通 `unit Base {}`，
调用写 `Base::swap<i32>(...)`，不能写成 `Base<i32>::swap(...)`。

更完整的语法、模块导入和边界说明见 [语言文档](language.md)。

普通输出统一使用标准库宏。格式化当前只支持 `{}` 占位符，字面量花括号写成 `{{` / `}}`；
浮点数不支持精度格式串，因此关联泛型示例中的 `80.0` 输出为 `80`。
`println!()` 输出空行，`print!` 不自动换行。详见 [格式化说明](formatting.md)。
