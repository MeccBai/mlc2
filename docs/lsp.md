# 简易 LSP

构建所有可执行文件后启动独立的 stdio 服务：

~~~powershell
cargo build --bins
mlc-lsp
~~~

编辑器客户端启动 mlc-lsp，不需要通过 mlc build。stdout 只用于 JSON-RPC，
不可混入命令行提示。当前依赖与构建器相同的 LLVM-C 运行库来获取目标数据布局。

本版提供四项功能：

- 悬停显示推断类型，并为没有显式类型的变量提供类型提示。
- 调用实参前显示参数名。
- 跳转到变量、参数、函数以及 unit、enum、union 的声明；支持跨文件类型和函数。
- 基于 lexer 和语义绑定提供 semantic tokens。

只分析磁盘上已保存的文件。didChange 后暂停该文件及依赖它的快照，didSave 后重新
分析；已打开文件的初始内容与磁盘不一致时同样暂停。lexer 或 parser 失败直接返回
空的语义结果；语义检查失败时保留已有结果，并通过 textDocument/diagnostic 返回结构化诊断。
修复保存后会清除已解决的报错。

同一份分析快照供四项功能共用，磁盘依赖改变或保存通知会使缓存失效。
不提供补全、重命名、引用查找或宏展开位置跳转。诊断采用 LSP 拉取模式，
VS Code 客户端会自动展示报错。类型使用短名；const 绑定显示 const，val 绑定显示普通类型。
函数悬浮展示签名；宏参数名来自原始声明，不显示展开后的内部名称。
CTFE 只检查调用规则，不安装求值执行器，不生成对象或动态库，也不做后端优化。
依赖实际求值才能确定的信息，首版不提供结果。

位置使用 UTF-16 行列，文件 URI 支持百分号编码的 UTF-8 路径。实现协议能力见
[LSP 3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)。

## todo()

内建 todo() 接受零个参数，可填入有预期类型的表达式位置：

~~~m2
unit Item { pub value: i32; };

func load() -> Item {
    return todo();
}

func main() -> i32 {
    var result: f64 = todo();
    return 0;
}
~~~

它不产生真实的替代值。正常编译后，一旦执行到该位置，就打印当前源文件、行、列
和 not implemented (todo)，然后调用退出接口以 -1 结束进程。无需 import std。
退出码在不同操作系统上可能表现为不同的非零整数。编辑器分析只检查它，不执行它。
