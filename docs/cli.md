# CLI

`build` 和 `fmt` 在交互终端的 stderr 显示分阶段进度条，结束后输出总耗时，
失败也输出耗时。重定向或管道中不输出动态进度控制字符，保留普通结果摘要。
项目多目标构建的耗时和统计累计计算。

Build 的缓存命中率为 Cached / (Cached + Built)，包含泛型实例对象的缓存检查，
不把预编译声明算成命中；链接复用次数和声明加载数单独列出。没有完成对象缓存
检查时显示 N/A。该指标不是前端分析缓存率，当前前端仍会执行分析和 IR 生成。
Fmt 不存在缓存命中率，显示实际写回与无需修改的文件数。

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
cargo build
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
mlc symbols lib/universal/c_std/io.h --module c_std::io --triplet universal  # 输出同名 io.sym
mlc symbols c_api.toml -o objects/universal/c_api.sym
mlc symbols native.h       # 固定宽度 C 声明 → native.sym
mlc symbols native.h --module c_std::native --triplet x86_64-pc-windows-msvc
```

`run` 必须指定源码文件或 target；仅支持 bin，构建失败时不会运行旧产物。
程序继承当前终端的输入输出和工作目录，退出码由 mlc 原样返回。
不再支持 `mlc main.m2` 形式。`ll`、`symbols` 和 `example` 命令保持独立。

示例独立维护在 `crates/examples`（`mlc_examples` crate），目前有 25 个，
按基础、函数与泛型、类型与资源分组；每份源码包含英文说明、运行命令和预期输出。
建议先运行 `mlc example list`，再按列表顺序学习。示例目录见 [示例指南](examples.md)。
源码编译时内嵌到 mlc，安装后不需要额外部署示例目录；example 命令不读取项目或全局配置。
打印出的源码保存为 `.m2` 后可以正常编译，普通输出使用 std::print! / std::println!；标准库示例展示 String、format! 与 Display，C 互操作单独展示。

`symbols` 将受限 `.h` 或 `Object = "None"` 的 TOML 声明转为 `.sym`。
头文件只接受完整具名 struct、函数原型和明确宽度类型（int32_t、uint8_t 等），
拒绝 int/long/char/size_t、预处理、typedef、属性、函数体和 struct 按值参数/返回。
不允许 #include，固定宽度名字由解析器直接识别。完整限制见
[ffi_parser](../crates/ffi_parser/README.md)。源码不再允许无 body 的 C ABI 声明，
必须 import 符号表；带 body 的 C ABI 定义仍可导出给 C 调用。
TOML 沿用原有字段和 `$none` 表示法。
该命令不读取项目或全局配置，不编译或链接；`-o` 在此命令中表示输出文件。
普通 import 只读取源码和二进制 `.sym`，不直接读取声明 TOML 或头文件。
`.sym` 和泛型 `.mg` 使用带格式版本头的 MessagePack，结构变更时必须更新格式版本。
现有旧 TOML 构建缓存需重新生成；Project.toml 和用户配置仍使用 TOML。

Type 仅为 bin/static/shared。project target 的 Triplet、LibDirs、LibCacheDir
覆盖项目与全局配置；CLI 的 --triplet、--lib-dir、-o 优先于配置。
可用 --config 指定全局配置文件；-j/--jobs 控制 LLVM object 任务数量。

## 当前链接工具

编译使用 MLC 前端与 llvm-sys，链接直接调用 LLD，不再调用 clang 驱动。
`src/driver/link/targets.rs` 为每个支持的 triplet 绑定 linker 和有序参数表。
Windows 默认使用 `x86_64-pc-windows-msvc` 和安装目录 tools 下的 `lld-link.exe`。
显式选择 GNU 目标时仍使用 `ld.lld.exe` 和匹配的 MinGW 运行库；其他 triplet
在补齐参数表及运行库前会明确报错。
`--linker` 覆盖链接器路径，`--link-arg` 直接传递原生链接参数（不使用 `-Wl,`）。
静态库使用 tools/llvm-ar，支持 `--archiver`。工具通过进程参数数组调用，不经过 shell。

安装布局：

```text
mlc.exe
LLVM-C.dll
tools/
  ld.lld.exe
  lld-link.exe
  llvm-ar.exe
lib/                         # universal/<triplet> 源码
objects/                     # universal/<triplet> 符号和对象缓存
runtimes/
  x86_64-pc-windows-gnu/
    crt2.o, dllcrt2.o, crtbegin.o, crtend.o
    libmingw32.a, libmingwex.a, libmoldname.a, libmsvcrt.a
    libclang_rt.builtins-x86_64.a
    libkernel32.a, libadvapi32.a, libshell32.a, libuser32.a
```

LLVM-C.dll 放在 mlc.exe 旁边；链接库按可执行文件位置查找，
不依赖 PATH。当前参数表不添加 libunwind 或 C++ 运行库，暂不支持异常展开。
EXE 使用 crt2.o，DLL 使用 dllcrt2.o 和 --shared；均保留 CRT 初始化。
链接缓存包含目标 runtimes 目录内文件内容，更新运行库会触发重新链接。

默认 triplet 跟随编译器自身的平台与架构，Windows 使用 `<arch>-pc-windows-gnu`。
优先级为 CLI `--triplet` > 项目选中 target 的 Triplet > 项目 Triplet > 系统默认。
目标选择不代表已配置该目标的原生链接参数表。

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

开发部署脚本使用环境变量 `MLC` 指定安装目录（本机已设置为 `D:\mlc`）：

```powershell
pwsh -NoProfile -File ./scripts/build_copy_debug.ps1
pwsh -NoProfile -File ./scripts/build_copy_release.ps1
```

脚本构建对应 profile，仅在构建成功后将生成的 mlc.exe 复制到 `$env:MLC`。
支持 Cargo 自定义 target 目录；已有会话未刷新环境变量时，也会读取用户级 MLC 设置。
安装目录中的 LLD、runtime 和 lib 需预先配置。

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
MSVC 链接环境配置完成；Windows 运行测试也统一生成 MSVC 对象并直接调用 lld-link。

## Debug and release builds

Use `--default` (neither optimization nor debug information), `--debug` (debug information only), `--release` (optimization only), or `--rel-with-info` (both). These flags are mutually exclusive. Project and target tables accept the matching `Mode = "default"`, `"debug"`, `"release"`, or `"rel-with-info"`; explicit flags take precedence. `--optimize` / `--no-optimize` override the optimization setting only. Standalone source files ignore Project.toml. The default is no optimization and no debug information. See [basic debugging information](debug-info.md) for PDB output and stepping rules.

构建目录按模式隔离：对象为 `build/objects/<mode>/<triplet>/`，IR 为 `build/ll/<mode>/<triplet>/`，最终产物为 `build/<mode>/main.exe`（PDB 同目录）。`-o` 替换整个 build 根目录。

标准库只使用 debug/release 两档。默认映射：default/debug → debug，release/rel-with-info → release。可在 Project.toml 的 `[Project]` 或 target 中写 `std_type = "debug"` 或 `std_type = "release"`，target 优先，没有对应 CLI 选项。标准库缓存为 `<安装根>/objects/<std_type>/<triplet>/`；显式 LibCacheDir 同样追加 `<std_type>/<triplet>`。纯声明 `objects/universal/` 保持共享。

通用固定宽度 C 声明可使用 `mlc symbols portable.h --triplet universal --module c_std::portable`，安装到 `objects/universal/`。这不会放宽头文件语法限制；ABI 不同的接口仍必须绑定具体 triplet。新增常用 C API 的模块和限制见 [c_std](c_std.md)。
