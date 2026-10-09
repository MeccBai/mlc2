# CLI

```powershell
cargo build
target/debug/mlc.exe --help
target/debug/mlc.exe tests/fixtures/hello_world.m2 --lib-dir lib -o build
./build/hello_world.exe
```

`--lib-dir lib` 用于开发仓库布局；正式安装默认使用 mlc 可执行文件旁的 lib。
命令读取当前目录的 Project.toml（兼容 project.toml）和 ~/.mlc/config.toml。
`-o` 是输出目录，不是可执行文件名。直接输入源码时输出名使用源码文件名。

```powershell
mlc main.m2 -o build
mlc build          # 构建项目中全部 target
mlc build app      # 构建名为 app 的 target
mlc lib.m2 --type static
mlc lib.m2 --type shared
mlc example list          # 列出内嵌示例
mlc example hello-world   # 仅打印源码，不编译或运行
mlc example generics
mlc ll main.m2             # 仅生成 LLVM IR，默认 build/ll
mlc ll main.m2 -o ir       # 指定 .ll 输出目录
mlc symbols lib/universal/c_std/io.toml  # 输出同名 io.sym
mlc symbols c_api.toml -o lib/universal/c_api.sym
```

示例独立维护在 `crates/examples`（`mlc_examples` crate），按主题拆分为
Hello World、变量、数组、控制流、函数、泛型、unit、引用和内存管理。
源码编译时内嵌到 mlc，安装后不需要额外部署示例目录；example 命令不读取项目或全局配置。
打印出的源码保存为 `.m2` 后可以正常编译，依赖当前提供的 C printf 声明与 std::mem。

`symbols` 仅转换 `Object = "None"` 的手写声明，沿用原有 TOML 字段和 `$none` 表示法。
该命令不读取项目或全局配置，不编译或链接；`-o` 在此命令中表示输出文件。
普通 import 只读取源码和二进制 `.sym`，不直接读取声明 TOML。
`.sym` 和泛型 `.mg` 使用带格式版本头的 MessagePack，结构变更时必须更新格式版本。
现有旧 TOML 构建缓存需重新生成；Project.toml 和用户配置仍使用 TOML。

Type 仅为 bin/static/shared。project target 的 Triplet、LibDirs、LibCacheDir
覆盖项目与全局配置；CLI 的 --triplet、--lib-dir、-o 优先于配置。
可用 --config 指定全局配置文件；-j/--jobs 控制 LLVM object 任务数量。

## 当前链接工具

编译使用 MLC 前端与 llvm-sys，不调用 clang 编译源文件。
链接使用 clang 驱动，由它配置 CRT/SDK、启动代码和库路径；
triplet 显式传入驱动，MSVC 使用 `-fuse-ld=lld -B <mlc目录>`，
GNU 使用 `-fuse-ld=lld --ld-path=<完整路径>` 指定同目录的 LLD：
MSVC 使用 `lld-link.exe`，GNU 使用 `ld.lld.exe`，不再从 PATH 搜索 LLD。
运行前会检查文件存在；MSVC 驱动不使用 `--ld-path`，避免其 unused warning。
通过 --linker 指定驱动、--link-arg 添加参数。静态库使用 llvm-ar，可用
--archiver 指定。两者均通过进程参数数组调用，不经过 shell。

未指定 triplet 时，根据 mlc 可执行文件自身的平台与架构自动选择：Windows 使用
`<arch>-pc-windows-gnu`；Linux 保留对应 GNU / musl triplet；macOS 使用
`<arch>-apple-darwin`（支持 x86_64 与 aarch64 目标初始化）。
优先级保持为 CLI `--triplet` > 项目选中 target 的 Triplet > 项目 Triplet > 系统默认。
交叉编译 mlc 时采用 Cargo TARGET 而不是构建机器 HOST，因此默认值跟随部署平台。

x86_64 Windows 默认 target 为 `x86_64-pc-windows-gnu`，默认链接驱动为
`x86_64-w64-mingw32-clang`（LLVM-MinGW/UCRT，需要在 PATH 中），
并在 mlc 同目录提供 `ld.lld.exe`。目标库及对象目录也按 GNU triplet 分开。
MSVC target 仍可显式指定；同时用 `--linker clang` 选择 MSVC 驱动，
并在 mlc 同目录提供 `lld-link.exe` 和相应 MSVC CRT/SDK 环境。
bin 入口要求 main 无参数，返回 i32 或 void。
shared 和 static 均支持全局运行期初始化。

每个编译模块生成唯一的 C ABI `void global_init(void)` 入口，实际符号名记录在
该模块 TOML 的 `[Config].GlobalInit`。符号名由完整模块名编码生成，稳定且不依赖
临时文件编号。入口持有私有 bool：已完成则直接返回，否则先调用依赖入口，
再按声明顺序初始化自己的全局变量，最后设置完成标记。

bin 自动调用根入口；shared 在加载时通过 LLVM global constructors 调用根入口，
并导出它。static 不注册自动构造器，外部使用者须在使用库前显式调用根入口；
MLC 导入预编译模块时会根据 `GlobalInit` 自动接入这条依赖链。普通 C ABI 调用，
不要求调用者理解 MLC 内部 ABI。入口引用依赖和自身初始化函数，使链接器能从
静态归档中提取所需对象。

初始化按启动阶段同步串行约定执行，不使用 atomic，也不保证并发调用安全。
BuildPlan 已拒绝循环依赖，所以只需完成标记，不需要“初始化中”状态。
链接/归档先写临时文件，成功才替换最终产物，避免旧 archive 残留成员。

## 编排和缓存边界

开发部署脚本（默认目标目录 `F:\Develop\MLC`，可传入其他目录）：

```powershell
pwsh -NoProfile -File ./scripts/copy_debug_to.ps1
pwsh -NoProfile -File ./scripts/copy_release_to.ps1
```

脚本构建对应 profile，复制 mlc、LLD、唯一运行时 DLL `LLVM-C.dll` 和仓库 lib；保留目标目录
其他文件，不执行清空操作。clang 驱动仍从 PATH 获取。

Package 分析及 IR 生成在主线程完成，不把 Rc 送进构建线程；taskflowrs
调度各源码的 LLVM object 生成和缓存发布。声明文件仅 export，不生成空函数体。
所有新泛型实例集中生成到一个补充 object，避免多文件重复定义。
native 函数的 linkage name 按模块限定，c_abi 函数保留原名。

当前缓存键保守地包含整组 IR，以避免文件编号/实例归属变化造成错误命中。
因此第二次构建可复用 object，但仍会执行前端；修改一个文件可能使其他文件
重生成 object。这版优先实现正确闭环，细粒度增量和增量链接尚未接入。

Hello World 使用显式固定长度数组：

```text
var text:[i8:14] = "Hello World!\n";
text[13] = 0;
```

字符串按 UTF-8 字节写入，容量不得小于字节数；多余空间清零。
字符串本身不会额外添加 NUL，作为 C 字符串使用时仍应预留终止位置。

```powershell
cargo test --test cli
cargo test --test cli -- --ignored # 配齐同目录 LLD 与 MSVC CRT/SDK 后
cargo test --test hello_world
```

CLI 执行测试保留完整构建、运行与缓存断言，目前显式标为 ignored，等待
MSVC 链接环境配置完成；独立 hello_world 测试仍验证旧的 LLVM-MinGW/UCRT 环境。
