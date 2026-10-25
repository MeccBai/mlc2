# MLC2

**中文** | [English](README.en.md)

MLC2 是一个用 Rust 实现、以 LLVM 为后端的静态类型语言编译器。它同时提供项目构建、增量缓存、格式化和 C 声明导入工具。

项目起于编译器实验与本科毕业设计，目标方向是嵌入式开发。目前主要验证环境是 **Windows x64 / MSVC / UCRT**；语言、标准库和工具链仍在演进，不承诺稳定 ABI 或生产级兼容性。

## 看一眼语言

```rust
import std;

func<T> swap(a: $mut T, b: $mut T) {
    var temporary = $a;
    $a = $b;
    $b = temporary;
}

func main() -> i32 {
    var a = 10;
    var b = 20;
    swap(@mut a, @mut b);
    std::println!("a={} b={}", a, b);
    return 0;
}
```

泛型参数可以显式指定，也可以从实参推断。`@` / `@mut` 创建只读 / 可变引用，`$` 解引用。普通 `main` 不需要标记 C ABI。

## 已有能力

- **类型与表达式**：后置类型标注、类型推断、数组、具名聚合类型 `unit`、枚举、带标签的 `union` 和函数指针。
- **泛型与接口**：泛型约束、单态化、泛型实参推断，以及绑定到类型的 interface；支持编译期宏函数和异构参数包展开。
- **资源管理**：无 GC；`res` 所有权、移动与失效检查、自动递归析构。引用不等于所有权，也不是 Rust 式的完整借用检查系统。
- **标准库与 FFI**：`String`、`format!`、`print!`、`println!`，以及通过 `.sym` 导入的 C API 声明。标准库仍在扩展。
- **生成与优化**：文本 LLVM IR、原生对象文件、可执行文件 / 静态库 / 动态库；独立的 AST 优化 pass，包括常量折叠和死代码消除。
- **开发工具**：依赖图构建、对象与链接缓存、进度与耗时统计、带源码上下文的诊断、固定样式格式化和可选 Windows 调试信息。

## 快速开始

以下命令假设已有配置好的 MLC 安装，`mlc` 在 PATH 中，标准库、链接工具和运行库已就位。安装布局见 [分发说明](docs/distribution.md)。

把上面的程序保存为 `main.m2`：

```powershell
mlc run main.m2
# a=20 b=10

mlc build main.m2
# 默认产物：build/default/main.exe

mlc ll main.m2
# 仅生成 LLVM IR，不编译对象、不链接

mlc fmt main.m2
```

`mlc build main.m2` 按单文件入口构建，忽略项目配置，但仍解析 import。项目模式使用 `Project.toml`：

```toml
[Project]
Name = "hello"
Version = "0.1.0"
OutputDir = "build"

[[Project.Targets]]
Name = "app"
Entry = "main.m2"
Type = "bin"
```

```powershell
mlc build          # 构建项目全部 target
mlc run app        # 构建并运行指定 target
mlc fmt            # 格式化整个项目的 .m2 源码
mlc example list   # 查看内嵌示例
mlc example generics
```

构建模式为 `--default`、`--debug`、`--release`、`--rel-with-info`，产物按模式隔离。后两种模式开启优化，`debug` 和 `rel-with-info` 启用调试信息。MLC 的这些模式与构建编译器自身时的 Cargo profile 是两回事。

## C 声明导入

外部 C 函数不在 `.m2` 中写无 body 声明，而是先把受限头文件转换为 `.sym`：

```c
// native.h — 不需要也不允许 #include
struct Point {
    int32_t x;
    int32_t y;
};
int32_t abs(int32_t value);
void update(struct Point *point);
```

```powershell
mlc symbols native.h   # 生成 native.sym，随后可 import native;
```

只接受明确宽度的整数类型、支持的浮点类型、指针和受限 struct / 函数声明。`int`、`long`、`char`、`size_t`、预处理、typedef、inline 和扩展属性等会报错。当前不支持 C struct 按值参数或返回值；有实现体的 MLC 函数仍可使用 `#[c_abi]#` 导出。

详见 [FFI 解析器](crates/ffi_parser/README.md) 与 [C API 模块](docs/c_std.md)。

## 从源码构建

当前构建脚本针对 Windows 开发环境配置，**不是安装 Rust 后便可跨平台构建**：

1. Rust 工具链，支持 edition 2024。
2. 与 `llvm-sys 231` 配套的 LLVM 23.1 开发库。当前脚本固定从 `$env:SCOOP/apps/llvm-dev/current/lib/LLVM-C.lib` 链接，需要设置 `SCOOP`。
3. 编译器运行时需要 `LLVM-C.dll`，放在可执行文件旁或可被系统加载的位置。
4. 链接 Windows 程序需要 `lld-link` 和 MSVC/UCRT/Windows SDK 库；静态归档使用 `llvm-ar`。C 插桩测试还需要 `clang-cl`，仅用于编译 C 夹具，不作为链接驱动。

```powershell
cargo build --release
./target/release/mlc.exe --help

# 前端与格式化等组件的测试
cargo test -p mlc_syntax -p mlc_core -p ffi_parser -p mlc_formatter

# 含 LLVM 对象生成和原生运行测试，需要上述工具链
cargo test --workspace
```

从 Cargo 输出目录运行时，库路径不会自动指向仓库的 `lib`。在仓库根目录准备声明符号表后，可以通过 `--lib-dir lib` 使用标准库：

```powershell
$mlc = (Resolve-Path ./target/release/mlc.exe).Path
$library = (Resolve-Path ./lib).Path
Get-ChildItem $library -Recurse -Filter *.h | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($library, $_.FullName)
    $parts = $relative -split '[/\\]'
    $module = ($parts[1..($parts.Length - 1)] -join '::') -replace '\.h$', ''
    $output = Join-Path objects ([IO.Path]::ChangeExtension($relative, '.sym'))
    & $mlc symbols $_.FullName --module $module --triplet $parts[0] -o $output
    if ($LASTEXITCODE -ne 0) { throw "Header conversion failed: $relative" }
}
& $mlc symbols lib/universal/std/mem.toml -o objects/universal/std/mem.sym
if ($LASTEXITCODE -ne 0) { throw 'std::mem conversion failed' }
& $mlc ll main.m2 --lib-dir lib
```

仅执行 `cargo build` 不会生成完整安装包，也不会下载 CRT 或链接工具。运行 / 链接程序还需配置 `tools` 与 `runtimes`；开发环境也可用 `--linker lld-link` 显式选择 PATH 中的链接器。工具放置、库路径和打包见 [分发说明](docs/distribution.md)。

Windows 默认目标为 `x86_64-pc-windows-msvc`，GNU 目标可显式选择。其他目标的 LLVM 对象生成能力不等于已经具备完整的标准库、链接参数和运行环境；嵌入式工具链仍是后续工作。

## 编译流程与 workspace

```text
.m2 → Lexer / Chumsky → Temp AST → 类型与语义分析 → AST 优化
                                                 ↓
                         原生对象 / 链接 ← LLVM ← 文本 IR
```

| Crate | 职责 |
| --- | --- |
| `mlc_syntax` | 词法、Chumsky 语法解析、Temp AST、span 和源码行表 |
| `mlc_core` | 符号 / 类型、推断、语义检查、泛型实例化及资源语义 |
| `mlc_optimizer` | 分析后的 AST 优化 pass |
| `mlc_codegen` | AST 到 LLVM IR、ABI 适配与析构代码生成 |
| `mlc_builder` | 依赖调度、缓存 / `.sym` / `.mg`、LLVM 对象生成 |
| `mlc_formatter` | 复用语法层、保留注释的固定样式格式化 |
| `ffi_parser` | 受限固定宽度 C 声明解析 |
| `mlc_examples` | 内嵌语言示例 |
| `mlc` | CLI、跨模块编排和最终链接 |

## 文档

- [完整语法与示例](docs/language.md)
- [CLI 与构建模式](docs/cli.md)
- [示例指南](docs/examples.md)
- [标准 C API 声明模块](docs/c_std.md)
- [构建管线与缓存](docs/build_pipeline.md)
- [安装布局与分发](docs/distribution.md)
- [格式化规则](docs/formatting.md)
- [调试信息](docs/debug-info.md)
- [Workspace 设计](docs/workspace.md)

功能和语法仍会调整。报告问题时，请附最小 `.m2` / `.h` 示例、执行命令、目标 triplet 和完整诊断输出。
