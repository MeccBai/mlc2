# Workspace 分层

| Package | 位置 | 职责 |
| --- | --- | --- |
| `mlc_syntax` | `crates/syntax` | lexer、Chumsky parser、Temp AST、共享语法定义 |
| `mlc_core` | `crates/core` | Temp → AST、语义诊断、符号表和导出表 |
| `mlc_codegen` | `crates/codegen` | AST → IR、ABI 映射与生成诊断 |
| `mlc_builder` | `crates/builder` | 构建产物、路径管理、LLVM OBJ 输出和链接 |
| `mlc` | 根 package 的 `src` | 顶层编排，当前仍只进行前端分析 |

生产依赖没有环：core → syntax；codegen → core / syntax；builder → core / syntax；mlc 使用各层。
codegen 的开发依赖使用 builder 验证生成的 IR，但 codegen 的生产依赖没有 LLVM。

现有类型的字段和语义保持不变。Operator、ValueType 和 ImportModule 由 syntax 定义，
core 在原 AST 路径重新导出，Temp 与真 AST 仍共享同一类型，并非复制两套定义。

跨 crate 不能为外部类型添加固有方法，因此后端方法通过扩展 trait 提供：

```rust
use mlc_codegen::{AstIr, SymbolIr};
use mlc_builder::import::ImportFetch;
```

`llvm_func()` 属于 SymbolIr；旧的 AST `generate()` 占位接口属于 AstIr；
import 的 `fetch()` 占位接口属于 ImportFetch。import 加载与入口编排仍未实现，本次不扩展功能。

常量按职责分别放在各 crate 的 manifest.rs，顶层 `src/manifest.rs` 统一重新导出。
LLVM 的链接脚本仅属于 builder。原模块测试跟随模块迁移，parser 测试拆分在自己的 tests 目录。

## 验证

```powershell
cargo check --workspace
cargo test --workspace
cargo test -p mlc_syntax -p mlc_core
cargo check -p mlc_codegen
cargo run -p mlc
```

前端测试和 codegen 的普通编译检查不需要 LLVM；builder 测试、codegen 的 OBJ 验证测试和最终可执行程序需要现有 LLVM 安装。
