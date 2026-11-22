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
3. `mlc-build` 运行时需要 `LLVM-C.dll`，放在可执行文件旁或可被系统加载的位置；分发器 `mlc` 和格式化器 `mlc-fmt` 不加载 LLVM。三个 EXE 同目录部署。
4. 链接 Windows 程序需要 `lld-link` 和 MSVC/UCRT/Windows SDK 库；静态归档使用 `llvm-ar`。C 插桩测试还需要 `clang-cl`，仅用于编译 C 夹具，不作为链接驱动。

```powershell
cargo build --release --bins
./target/release/mlc.exe --help

# 前端与格式化等组件的测试
cargo test -p mlc_syntax -p mlc_core -p ffi_parser -p mlc_formatter

# 含 LLVM 对象生成和原生运行测试，需要上述工具链
cargo test --workspace
```

开发时将 MLC 指向仓库根目录，使用包注册表准备标准库：

```powershell
$env:MLC = (Get-Location).Path
./target/release/mlc.exe setup
./target/release/mlc.exe run crates/examples/sources/hello_world.m2
```

## 文档

- [语言使用](docs/README.md#语言使用)
- [编译器实现](docs/README.md#实现)

功能和语法仍会调整。报告问题时，请附最小 `.m2` / `.h` 示例、执行命令、目标 triplet 和完整诊断输出。
