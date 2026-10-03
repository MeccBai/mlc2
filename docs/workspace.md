# Workspace 分层

| Package | 位置 | 职责 |
| --- | --- | --- |
| `mlc_syntax` | `crates/syntax` | lexer、Chumsky parser、Temp AST、共享语法定义 |
| `mlc_core` | `crates/core` | Temp → AST、语义诊断、符号表和导出表 |
| `mlc_codegen` | `crates/codegen` | AST → IR、ABI 映射与生成诊断 |
| `mlc_builder` | `crates/builder` | 构建产物、路径管理、LLVM OBJ 输出和链接 |
| `mlc_examples` | `crates/examples` | 按主题组织的内嵌示例源码、索引与文本输出 |
| `mlc` | 根 package 的 `src` | CLI、前端与 IR 编排、构建及链接 |

生产依赖没有环：core → syntax；codegen → core / syntax；builder → core / syntax；mlc 使用各层。
codegen 的开发依赖使用 builder 验证生成的 IR，但 codegen 的生产依赖没有 LLVM。
examples 没有生产依赖，开发依赖 syntax 验证语法；顶层 mlc 测试负责示例的编译运行验证。

现有类型的字段和语义保持不变。Operator、ValueType 和 ImportModule 由 syntax 定义，
core 在原 AST 路径重新导出，Temp 与真 AST 仍共享同一类型，并非复制两套定义。

跨 crate 不能为外部类型添加固有方法，因此后端方法通过扩展 trait 提供：

```rust
use mlc_codegen::{IrGenerator, SymbolIr};
use mlc_builder::plan::BuildPlan;
```

`llvm_func()` 属于 SymbolIr；IR 通过 IrGenerator 生成，由顶层 driver 编排
各模块、泛型实例和初始化入口。import 的递归发现与依赖校验由 BuildPlan 负责，
加载预编译产物由 builder 的 artifacts 模块负责，不再保留未实现的扩展接口。

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
