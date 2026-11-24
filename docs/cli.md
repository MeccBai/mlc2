# CLI

CLI 由同目录的 `mlc`、`mlc-build`、`mlc-fmt` 组成，用户仍统一调用 `mlc`。
`fmt` 分发给 mlc-fmt，其余业务命令交给 mlc-build；`--help` 和 `--version` 不启动工作进程。

`mlc clean` 删除当前项目的整个构建输出目录（包括所有 profile）。默认是 `build/`，
存在 Project.toml 时使用 `Project.OutputDir`；`mlc clean -o out` 可显式指定。
目录不存在视为成功，不清理安装目录中的标准库缓存。为避免误删，拒绝项目根目录、
项目外路径、文件、目录链接以及含 Project.toml 或 .git 的目录。

`build` 和 `fmt` 在交互终端的 stderr 显示分阶段进度条，结束后输出总耗时，
失败也输出耗时。重定向或管道中不输出动态进度控制字符，保留普通结果摘要。
项目多目标构建的耗时和统计累计计算。

Build 的缓存命中率为 Cached / (Cached + Built)，包含泛型实例对象的缓存检查，
不把预编译声明算成命中；链接复用次数和声明加载数单独列出。没有完成对象缓存
检查时显示 N/A。
Fmt 不存在缓存命中率，显示实际写回与无需修改的文件数。

## 颜色与 JSON 输出

普通终端输出为成功状态、产物路径、警告和错误添加颜色。重定向时默认不输出颜色；
设置环境变量 `NO_COLOR=1` 可关闭颜色。

`--json` 是全局参数，可放在子命令之前或之后：

```powershell
mlc build main.m2 --json
mlc --json build
mlc run main.m2 --json
mlc fmt main.m2 --json
```

该模式向 stdout 输出一个 JSON 对象，关闭进度条、耗时摘要、ANSI 颜色和源码位置的
文本渲染。词法、语法、语义错误及 warning 都进入 `diagnostics`，不再打印源码片段。
成功构建同样输出 JSON；进程退出码仍保留成功、失败或被运行程序的退出状态。

| 字段 | 内容 |
| --- | --- |
| `version` | 输出结构版本，当前为 1 |
| `command` | 子命令名称；参数解析结果使用 cli |
| `success` / `exit_code` | 是否成功及进程退出码 |
| `artifacts` | 产物列表，每项包含 kind 和 path |
| `diagnostics` | 诊断列表，每项包含 severity 和 message |
| `messages` | 其他命令结果说明 |
| `run` | run 命令的程序 stdout、stderr、exit_code；其他命令为 null |

编译管线通过统一的 `Diagnostic` / `Report` 传递诊断，保留严重程度、消息、源码
快照和字节范围。扫描、语义检查、格式化和声明导入阶段不渲染提示；最外层 CLI
统一选择文本渲染或 JSON 序列化，warning 也遵循同一路径。

源码诊断另含 `file`、`span.start`、`span.end`、`line`、`column`。
span 是从 0 开始的 UTF-8 字节范围，结束位置不包含在内；行列从 1 开始，列按 Unicode
字符计算。文件不存在、链接失败等没有源码范围的错误只提供 severity 和 message。
`run --json` 将程序输出收进 run 字段，程序输出不会混入顶层 JSON；程序返回非零值时
success 为 false，退出码保持该值。

## 格式化

```powershell
mlc fmt main.m2     # 原地格式化单个文件
mlc fmt             # 递归格式化整个项目的 .m2 源码
```

不提供样式配置。项目根目录为当前目录或最近的包含 Project.toml 的父目录；
无配置时使用当前目录。跳过隐藏目录、符号链接、build、target、objects、tools、
runtimes，以及项目配置的输出/缓存目录。不加载 import，不要求语义分析成功。
单文件模式不读取项目配置。项目中任一文件存在词法/语法错误时，本次不写回任何文件。
所有输出先暂存，再逐文件原子替换；文件系统写入失败不保证整个项目回滚。
格式未改变的文件不会重写，保留时间戳。

```powershell
cargo build --bins
target/debug/mlc.exe --help
target/debug/mlc.exe build tests/fixtures/hello_world.m2 --lib-dir lib -o build
./build/hello_world.exe
```

`--lib-dir lib` 用于开发仓库布局；正式安装默认使用 mlc 可执行文件旁的 lib。
项目模式读取当前目录的 Project.toml（兼容 project.toml）。
参数以 `.m2` 结尾时作为源码入口，完全忽略项目文件，仍读取 ~/.mlc/config.toml。
`-o` 是输出目录，不是可执行文件名。源码模式的输出名使用源码文件名。

```powershell
mlc build main.m2 -o build  # 单文件入口，忽略项目配置
mlc build          # 构建项目中全部 target
mlc build main     # 构建项目中名为 main 的 target
mlc run main.m2    # 构建源码入口并运行
mlc run main       # 构建项目 target 并运行
mlc build lib.m2 --type static
mlc build lib.m2 --type shared
mlc example list          # 列出内嵌示例
mlc example hello-world   # 仅打印源码，不编译或运行
mlc example generics
mlc ll main.m2             # 仅生成 LLVM IR，默认 build/ll/<mode>/<triplet>
mlc ll main.m2 -o ir       # 指定 .ll 输出目录
mlc symbols packages/c_std/0.2.2/source/io.h --module c_std::io --triplet universal  # 输出同名 io.sym
mlc symbols c_api.toml -o declarations/c_api.sym
mlc symbols native.h       # 固定宽度 C 声明 → native.sym
mlc symbols native.h --module c_std::native --triplet x86_64-pc-windows-msvc
```

`run` 必须指定源码文件或 target；仅支持 bin，构建失败时不会运行旧产物。
程序继承当前终端的输入输出和工作目录，退出码由 mlc 原样返回。
不再支持 `mlc main.m2` 形式。`ll`、`symbols` 和 `example` 命令保持独立。

示例独立维护在 `crates/examples`（`mlc_examples` crate），目前有 30 个，
按基础、函数与泛型、类型与资源分组；每份源码包含英文说明、运行命令和预期输出。
建议先运行 `mlc example list`，再按列表顺序学习。示例目录见 [示例指南](examples.md)。
源码编译时内嵌到 mlc，安装后不需要额外部署示例目录；example 命令不读取项目或全局配置。
打印出的源码保存为 `.m2` 后可以正常编译，普通输出使用 std::print! / std::println!；标准库示例展示 String、format! 与 Display，C 互操作单独展示。

`symbols` 将受限 `.h` 或 `Object = "None"` 的 TOML 声明转为 `.sym`。
头文件只接受完整具名 struct、函数原型和明确宽度类型（int32_t、uint8_t 等），
拒绝 int/long/char/size_t、预处理、typedef、属性、函数体和 struct 按值参数/返回。
不允许 #include，固定宽度名字由解析器直接识别。完整限制见
[C 声明使用](c_std.md)。源码不再允许无 body 的 C ABI 声明，
必须 import 符号表；带 body 的 C ABI 定义仍可导出给 C 调用。
TOML 沿用原有字段和 `$none` 表示法。
该命令不读取项目或全局配置，不编译或链接；`-o` 在此命令中表示输出文件。
普通 import 可以加载源码、`.sym` 声明或内建模块。声明 TOML 和头文件需要先转换。
旧版本生成的构建缓存需要重新生成。

Type 仅为 bin/static/shared。project target 的 Triplet、LibDirs、LibCacheDir
覆盖项目与全局配置；CLI 的 --triplet、--lib-dir、-o 优先于配置。
可用 --config 指定全局配置文件。用 -j 或 --jobs 设置构建并发数。

## 链接选项

Windows 默认使用 MSVC 目标，也可用 `--triplet x86_64-pc-windows-gnu` 选择 GNU 目标。目标优先级为 CLI、target、项目、平台默认值。

用 `--linker` 或 `--archiver` 指定工具路径。`--link-arg` 直接传递链接参数。`--runtime-dir` 指向当前目标所需运行库的目录。安装要求见 [安装与分发](distribution.md)。

可执行目标需要一个无参数、返回 `i32` 或无返回值的 `main`。

## Debug and release builds

`mlc build <target>` 和 `mlc run <target>` 默认读取选中 target 的 `Profile`：

```toml
[[Project.Targets]]
Name = "app"
Entry = "src/main.m2"
Type = "bin"
Profile = "release"
```

可选值为 default/debug/release/rel-with-info。优先级：显式 CLI 模式 > target Profile >
target Mode > Project Mode > default。省略 Profile 的现有项目仍可使用 Mode；
显式 Profile="default" 也会覆盖旧 Mode。`mlc build` 构建全部 target 时分别使用各自配置。
显式 `.m2` 入口仍不读取 Project.toml。

Use `--default` (neither optimization nor debug information), `--debug` (debug information only), `--release` (optimization only), or `--rel-with-info` (both). These flags are mutually exclusive. Project and target tables accept the matching `Mode = "default"`, `"debug"`, `"release"`, or `"rel-with-info"`; explicit flags take precedence. `--optimize` / `--no-optimize` override the optimization setting only. Standalone source files ignore Project.toml. The default is no optimization and no debug information. See [basic debugging information](debug-info.md) for PDB output and stepping rules.

构建目录按模式隔离：对象为 `build/objects/<mode>/<triplet>/`，IR 为 `build/ll/<mode>/<triplet>/`，最终产物为 `build/<mode>/main.exe`（PDB 同目录）。`-o` 替换整个 build 根目录。

标准库构建模式默认随项目模式选择。可在项目或 target 设置 Std = "debug"、"release" 或 "None"，target 优先。"None" 关闭自动引入标准库。

通用固定宽度 C 声明可使用 `mlc symbols portable.h --triplet universal --module c_std::portable`，由 `mlc setup` 写入包内声明缓存。这不会放宽头文件语法限制；ABI 不同的接口仍必须绑定具体 triplet。新增常用 C API 的模块和限制见 [c_std](c_std.md)。
## Initialize a project and prepare libraries

```powershell
mlc setup                  # prepare registered Source packages for debug and release
mlc setup --release        # prepare only the release library variant
mlc setup --triplet x86_64-pc-windows-msvc

mlc init hello
cd hello
mlc build
mlc run hello
```

`init <project>` creates a new directory containing `Project.toml` and `src/main.m2` with a Hello World program. The executable target uses the directory name. Existing directories/files are never overwritten. A parent directory supplied as part of the path must already exist.

`setup` reads packages/packages.toml beneath MLC, converts all package .h/.toml declarations first, and compiles all .m2 modules into each package's .compiled directory. It ignores Project.toml/user configuration and invokes no linker or archiver. --triplet and profile flags restrict preparation; setup rejects --lib-dir and -o.

Missing imports report the actual source and symbol paths searched. Missing entries, declaration inputs, Project.toml, explicit config files and native tools/libraries include their expected paths. The optional default user configuration may still be absent.

## 编译期求值预算

单次 CTFE 求值默认最多执行 3 秒，递归和内部 const 调用共享这次预算；编译和链接所需时间不计入执行预算。超时会终止并回收求值子进程，报错定位到调用位置。

项目构建可在 Project.toml 顶层配置秒数：

~~~toml
[CTFE]
MaxTime = 3.0
~~~

必须是有限且大于零的秒数。3.0.0 不是合法 TOML 数值。显式 .m2 入口仍遵循原规则，
忽略 Project.toml，使用默认 3 秒。LSP 不执行 CTFE，因此不消耗此预算。


## Local package installation

Declare a library target with Type = "lib" (also accepts "static" or "shared").
Run mlc install <target> from its project directory. Installation always builds first;
failed builds do not publish a package. The default representation is Source.

~~~powershell
mlc install my_library
mlc install my_library --release
mlc install my_library --package-type static
mlc install my_library --package-type shared
~~~

Packages are installed beneath $MLC/packages/<target>/<Project.Version>/source/ for Source,
or <debug|release>/<static|shared>/ for binary representations.
The installation root falls back to the compiler directory when MLC is unset.
Project.Version must be an exact major.minor.patch version. An installed representation
is immutable; publish changes with a new version. Source installations include the
imported project sources and successful build artifacts in .compiled/.

The installation's packages/packages.toml records available representations, entries, binary
triplets and transitive requirements. Source is registered once at [Packages.Source]. Each declared Debug/Release profile must contain Static or Shared; both can coexist. Default/Debug consumers use debug binaries;
Release/RelWithInfo consumers use release binaries. The Source tree is shared by all profiles. Source packages are compiled and
cached separately in $MLC/cache/artifacts/ for the consumer's triplet and chosen linkage.

~~~toml
[Project]
Name = "application"
Version = "1.0.0"
OutputDir = "build"

[[Project.Targets]]
Name = "app"
Entry = "src/main.m2"
Type = "bin"
Profile = "default"

[[Project.Targets.Requires]]
Name = "my_library"
Version = "1.5.1"
Type = "Source"
LinkType = "Shared"
~~~

Import packages by their target name. Source defaults to Static linkage when LinkType
is omitted; it also accepts Shared. Type = "Static" and Type = "Shared" select their
fixed representation and reject LinkType. Binary packages must match the target triplet.
Shared runtimes are copied beside the resulting executable. Missing versions/profiles,
conflicting requirements and dependency cycles are errors. Only direct dependencies
can be imported by a project module. Versions resolve from the local catalogue; there
is no network package download or version-range resolution.

std and c_std use the same package layout. The old installation lib/ and objects/ directories are not searched.
