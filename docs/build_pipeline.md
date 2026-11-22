# 构建流水线（当前实现）

实际 CLI 编排已接入，命令及当前链接工具约束见 [CLI](cli.md)。
本文主要描述可复用的低层构建接口；CLI 的 Package 分析在主线程完成。

## 安装布局与查找顺序

标准包从 MLC 指定的 packages/packages.toml 解析，源码在 packages/<name>/<version>/source/。
std/c_std 默认可导入，子模块相对于各包入口目录定位。平台声明位于 source/targets/<triplet>/。
旧安装目录 lib/、objects/ 不参与标准库查找。具体目录和 setup 行为见 [安装分发](distribution.md)。
普通库依赖通过 Project.Targets.Requires 指定；解析检查版本、profile、表示方式、triplet、依赖环与冲突。
递归扫描在启动线程池前返回导入环；重复 import 只保留一条依赖边。

## 三层接口

| 层 | 入口 | 职责 |
| --- | --- | --- |
| 持久化 | `mlc_syntax::serialization`、`mlc_builder::artifacts` | Temp 序列化、二进制元数据、内容哈希、发布与校验 |
| 构建计划 | `BuildPlan::discover(entry, resolver)` | 文件发现、索引图、反向依赖、环检测、依赖优先顺序 |
| 调度执行 | `schedule::build(plan, options, compile)` | taskflowrs 依赖任务、缓存决策、失败阻断、LLVM object 输出 |

`compile` 接收 `CompileRequest`，包含当前源码节点、直接依赖的
`Artifact` 和构建选项。成功返回 `CompileOutput { ir: Some(ir), global_init: None }`
（有模块初始化入口时填写 `global_init`），
构建层在该工作线程中创建 LLVM target machine 并产生 object；
显式返回 `ir: None` 表示只发布声明，不产生 object。

工作线程之间只传递声明、Temp 模板和路径，不传递带 Rc 的 AST 或符号表。
AST、PackageSymbolTable 和 IR generator 应在回调内部创建和释放。
端到端回调和 Package 跨文件生成测试见 `tests/build_pipeline.rs`。
构建回调示例暂仅编译独立源码，实际依赖产物装载仍由编排器提供。

taskflowrs 的任务没有业务返回值，因此这里使用每节点一个
`OnceLock<NodeResult>` 回收 `Result`。依赖未满足的节点不进入执行队列，
不会占用工作线程等待前驱。某个任务失败会阻断其使用者，其他任务继续。

## 产物与缓存

项目对象位于 build/objects/<profile>/<triplet>/；标准包缓存位于自身 source/.compiled/objects/<profile>/<triplet>/，
头文件和 TOML 声明转换到 source/.compiled/declarations/<triplet>/。写入失败时可以使用项目内缓存，
不会搜索旧安装对象目录。普通 Source 包的独立原生库缓存位于 MLC/cache/artifacts/。

- `.sym`：MessagePack 二进制，保存版本、模块名、源文件路径、目标、构建键、依赖指纹和声明。
- `.obj` / `.o`：LLVM 输出，根据 target 选择后缀。
- `.mg`：存在泛型模板时保存 TempModule 的 MessagePack 二进制表示。

`.mg` 暂时保存整个翻译单元，包含私有 helper、using、全局变量和函数体，
以保留模板的依赖闭包。普通函数声明只保存 symbol，不在声明表内保存 body。
inner 声明不等于对外可见声明，不能直接拼接进公共搜索空间。

序列化保留 Span 和现有类型字段，不持久化运行期 arena index 或 Rc。
二进制文件带有 magic 和版本头；旧格式构建缓存需要重新生成。
手写 C 声明仍可维护为 TOML，通过 `mlc symbols io.toml` 生成 `io.sym` 后导入。
也支持 `mlc symbols native.h`：受限固定宽度 C 声明由 ffi_parser 处理，生成
Object=None 符号表；默认目标为宿主 triplet，ABI 一致的通用接口可显式选择
`--triplet universal`。普通源码不允许无 body 的 C ABI 声明。
普通 import 不直接读取 TOML 或 `.h`；Project.toml 和全局配置格式不变。
TOML 没有 null：编译器生成格式用保留表 `{ "$none" = true }`
表示 `None` 和 unit 值；因此这不是最终面向人工编写的精简 C FFI 格式。

缓存键包含源文件哈希、目标 triplet、调用方提供的 compiler_id、
compiler_options 和依赖指纹。调用方必须让 compiler_id 对编译器语义变化
敏感，并在 compiler_options 中完整记录影响编译的设置。

普通函数体变化只使自身重编译。公开声明或泛型闭包变化会向使用者传播。
目前哈希基于 Temp 声明而不是解析后的类型布局，所以声明哈希保守地包含
依赖指纹，避免间接布局变化漏编译；这可能产生额外的传递性重编译。
Span 不参与语义哈希，但源文件任何变化都会改变自己的 FileHash。
CLI 不再把整个计划的 IR 指纹混入所有文件的 compiler_options；每个源码节点
仅记录自身 IR 指纹，作为文件 ID 和降低结果变化的安全兜底。它不会直接污染其他节点。
缓存检查仍按依赖顺序推进：自身源码、自身 IR 或直接依赖的符号/泛型哈希变化才重编。
依赖声明哈希包含其依赖指纹，因此公开声明和泛型闭包变化仍可能保守地链式传播。
前端仍对整个计划执行分析与 IR 生成；此缓存减少的是 LLVM OBJ 生成和链接工作。

泛型实例 OBJ 按其 IR、triplet 和编译器版本独立复用。最终链接产物按所有 OBJ 内容、
链接工具及参数和搜索环境检查缓存，输出文件也校验实际内容；变化时仅重新链接。
额外 link-arg 可能引用未跟踪的库或脚本，存在这些参数时保守地每次重新链接。
缓存标记保存在产物旁的 `.cache` 文件，损坏或丢失会自动重新生成。

缓存和产物兼容性统一由 `mlc_manifest::BUILD_NUMBER` 决定，不使用 Cargo 版本号。
历史名称 `FORMAT_VERSION` / `FormatVersion` 仅作为该 build 号的常量别名和序列化字段名，
没有独立的版本计数器。修改编译语义或序列化布局时递增 build 号，并重新生成 `.sym` / `.mg`。
`Object` 表示 `Source`、`None` 或 `Only` 模式，
object 文件名独立记录在 `ObjectFile`。`Source` 校验源文件、声明、object、
模板文件和模板语义哈希；`Only` 不需要源文件，仅校验模板哈希；
`None` 仅声明，不校验任何哈希，也不携带 object 或模板。
跳过哈希校验不跳过格式版本、引用路径和文件存在性检查。
依赖指纹从实际声明计算；object 的运行期内容哈希用于链接增量判断，
即使 `Only` 不校验记录的 ObjectHash，也不会漏掉二进制变化。
源文件缓存损坏会视为 cache miss；只有预编译声明的损坏会报告节点失败。
object 和模板先写入，manifest 最后原子替换。不再被引用的旧产物暂不清理。

`BuildReport.link_key` 包含 object 内容哈希：即使普通函数体变化未改变
声明哈希，也可以触发后续重链接。`build()` 本身暂不执行入口包装或链接。

## 路径配置

`PathResolver::load(executable, project_root, target_name)` 默认读取
`~/.mlc/config.toml` 和项目 `Project.toml`（兼容 `project.toml`）。
`Triplet`、`LibDirs` 和 `LibCacheDir` 分别按 target → project → global/default
选择；triplet 不从全局配置读取，未指定时使用编译器宿主 triplet。
配置的库根目录和缓存根目录均自动追加 `<triplet>`。
同一级 `LibDirs` 数组按顺序查找，显式配置替换下一级目录列表。
当前导入文件所在目录仍优先于所有库目录。
标准库通过注册表定位，各包内的 C 声明由 setup 预处理；实际实现和链接库由目标 C 运行时提供。
项目配置的相对路径基于项目根目录，全局配置的相对路径基于配置文件目录。

全局配置示例：

```toml
LibDirs = ["D:/extra/sources"]
LibCacheDir = "D:/mlc/cache"
```

项目配置可在 `[Project]` 或 `[[Project.Targets]]` 内设置：

```toml
Triplet = "x86_64-pc-windows-msvc"
LibDirs = ["vendor/lib"]
LibCacheDir = "build/lib-cache"
```

未配置 `LibCacheDir` 时优先原地写库缓存；配置后优先写入指定缓存目录。
`BuildPlan::discover_with_paths()` 固定产物位置，`BuildOptions.target` 必须与
resolver 的 triplet 一致。写入探测或实际发布失败会尝试回退，警告收集到
`BuildReport.warnings`；所有位置都失败时正常返回节点错误。

## Core 接入与入口编排边界

Core 已通过 ResolveContext 接通 path、resolve_type、跨 arena 类型操作和
函数、接口、unit 的泛型实例化。调用方只需通过 `package.set_imports()`
给出允许搜索的依赖 FileId，再调用 AST 的 export/analysis。
具体可见性、inner 访问和实例归属规则见 [Core 跨文件解析](core_resolution.md)。

构建回调仍需将便携声明及模板装载到自己的 Package 中，分配文件身份，
设置各文件的 import 范围，并按需要收集新实例 body。依赖产物中的 Temp
会重新解析，不把其他 Package 的运行期 index 直接复制过来。
这个编排入口和最终链接没有在这里替代调用方实现。

运行期 arena 文件身份不能直接持久化为外部符号。当前 CLI 的全局初始化符号
按完整模块名稳定编码，不依赖临时文件编号；自身 IR 指纹也参与对象缓存校验。

## 验证

```text
cargo test --workspace
cargo test --test hello_world -- --nocapture
cargo test --test build_pipeline
```

覆盖 Temp 往返、源码到 object、增量缓存、损坏恢复、模板变化、
传递依赖变化、独立任务失败隔离、目录优先级、重复依赖和环检测。

Windows 的 `hello_world` 测试由测试 helper 从声明输入建立隔离安装目录，
编译 `tests/fixtures/hello_world.m2`：构造 i8 数组、手动将最后一项写为 0，
将第一项的引用传给 printf。MLC/LLVM 生成 object 后，使用 PATH 中的
MSVC/UCRT `lld-link` 直接链接，复用 CLI 参数表和库路径发现逻辑。
可用 `MLC_TEST_LLD_LINK` 指定链接器；仅 C 插桩夹具通过 clang-cl /c 编译，
可用 `MLC_TEST_CLANG_CL` 指定 C 编译器，不再使用 Clang 作为链接驱动。
测试 CRT 优先使用 `MLC_TEST_RUNTIME_DIR`，其次测试程序安装目录或 PATH 中
MLC 安装的 runtimes/<triplet>，最后使用现有 MSVC/SDK 自动发现。
运行结果必须为退出码 0、stdout `Hello World!\r\n`、stderr 为空。
该测试不跳过缺失工具，也不替代正式的入口/链接参数编排。

## 生成产物与首次检出

Git 保存标准库的 `.m2`、声明 `.toml` 和受限 `.h` 输入；`.sym`、`.obj`、`.mg` 都是生成产物，不提交到仓库。
测试 helper 会自动生成隔离声明。手动运行示例前，可使用打包脚本建立完整安装：

```powershell
cargo build --bins
$env:MLC = (Get-Location).Path
./target/debug/mlc.exe setup
```

setup 转换注册包中的所有声明输入，并预编译所有源码模块。工具及运行库配置见 [安装分发](distribution.md)。
标准库 `.m2` 的对象文件与模板缓存由正常构建按需生成。

标准库只提供 debug/release 两档，Project.toml 的 `[Project]` 或 target 可设置 `std_type = "debug"` / `"release"`；target 优先。省略时 default/debug 使用 debug，release/rel-with-info 使用 release。该设置没有 CLI 选项，单文件入口继续忽略 Project.toml。

`mlc setup` prepares registered Source packages, converts their declaration inputs, and writes artifacts inside each package.

## CLI 构建后端

### 当前链接工具

编译使用 MLC 前端与 llvm-sys，链接直接调用 LLD，不再调用 clang 驱动。
`src/driver/link/targets.rs` 为每个支持的 triplet 绑定 linker 和有序参数表。
Windows 默认使用 `x86_64-pc-windows-msvc` 和安装目录 tools 下的 `lld-link.exe`。
显式选择 GNU 目标时仍使用 `ld.lld.exe` 和匹配的 MinGW 运行库；其他 triplet
在补齐参数表及运行库前会明确报错。
`--linker` 覆盖链接器路径，`--link-arg` 直接传递原生链接参数（不使用 `-Wl,`）。
`--runtime-dir` 覆盖当前目标的运行库目录（直接指向含 `.lib` 或 CRT 对象的目录，
不再追加 triplet）；链接与运行库内容缓存校验均使用此目录。
仓库开发测试可指定 `--linker tools/lld-link.exe --runtime-dir runtimes/x86_64-pc-windows-msvc`。
静态库使用 tools/llvm-ar，支持 `--archiver`。工具通过进程参数数组调用，不经过 shell。

安装布局：

```text
mlc.exe
mlc-build.exe
mlc-fmt.exe
LLVM-C.dll
tools/
  ld.lld.exe
  lld-link.exe
  llvm-ar.exe
packages/packages.toml                # 安装包注册表
packages/                    # 版本化的源码、静态库、动态库
runtimes/
  x86_64-pc-windows-msvc/
    msvcrt.lib, vcruntime.lib, ucrt.lib, kernel32.lib
    legacy_stdio_definitions.lib, oldnames.lib
  x86_64-pc-windows-gnu/
    crt2.o, dllcrt2.o, crtbegin.o, crtend.o
    libmingw32.a, libmingwex.a, libmoldname.a, libmsvcrt.a
    libclang_rt.builtins-x86_64.a
    libkernel32.a, libadvapi32.a, libshell32.a, libuser32.a
```

LLVM-C.dll 放在 mlc.exe 旁边；链接库优先使用安装目录 runtimes。
MSVC 缺库时还会查询 LIB、Build Tools 和 Windows SDK；完整随包库不要求系统 SDK。
当前参数表不添加 libunwind 或 C++ 运行库，暂不支持异常展开。
MSVC EXE 使用 /subsystem:console，DLL 使用 /dll；GNU EXE 使用 crt2.o，
DLL 使用 dllcrt2.o 和 --shared。两种配置均保留 CRT 初始化。
链接缓存包含目标 runtimes 目录内文件内容，更新运行库会触发重新链接。

默认 triplet 跟随编译器自身的平台与架构，Windows 使用 `<arch>-pc-windows-msvc`。
优先级为 CLI `--triplet` > 项目选中 target 的 Triplet > 项目 Triplet > 系统默认。
目标选择不代表已配置该目标的原生链接参数表。

bin 入口要求 main 无参数，返回 i32 或 void。
shared 和 static 均支持全局运行期初始化。

每个编译模块生成唯一的 C ABI `void global_init(void)` 入口，实际符号名记录在
该模块 `.sym` 元数据的 `Config.GlobalInit`。符号名由完整模块名编码生成，稳定且不依赖
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

### 编排和缓存边界

运行 `./scripts/deploy.ps1 -Profile debug` 或 `-Profile release` 构建并部署到 `$env:MLC`；`-SkipBuild` 使用已有产物。安装布局见 [安装分发](distribution.md)。标准库与普通库统一放入版本化的 packages 目录。

Package 分析及 IR 生成在主线程完成，不把 Rc 送进构建线程；taskflowrs
调度各源码的 LLVM object 生成和缓存发布。声明文件仅 export，不生成空函数体。
所有新泛型实例集中生成到一个补充 object，避免多文件重复定义。
native 函数的 linkage name 按模块限定，c_abi 函数保留原名。

每个源码节点按自身源码、IR 和直接依赖的符号/泛型指纹检查 object 缓存。
普通函数体修改通常只重编自身；公开声明或泛型闭包变化可保守地链式传播。
前端仍重新分析和生成 IR；泛型实例对象和最终链接分别检查缓存。
这里的链接复用不是增量链接器内部补丁，详细边界见 [构建流水线](build_pipeline.md)。

普通 Hello World 推荐 `import std;` 后调用 `std::println!("Hello World!");`。
C 字符串互操作测试另外使用显式固定长度数组：

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

### CTFE 缓存

编译期求值缓存独立放在 `build/ctfe/<host-triplet>/<call-hash>/`，
与 `build/objects/` 和 `build/<profile>/` 并列。`-o custom` 时改用 `custom/ctfe/`。
同一目标的 default、debug、release、rel-with-info 共用缓存，`--optimize` 和
`--no-optimize` 也不改变 CTFE 的优化策略。

CTFE 固定开启全部 AST 优化和 LLVM O3，关闭调试信息。目录只按实际执行 CTFE 的宿主分类；目标信息仍保留在缓存校验记录中，避免
交叉编译时混用不同的数据布局。源码、参数、编译器或布局变化仍会使对应缓存失效。

### 内建模块

`std::bit` 和 `std::mem` 是构建计划中的虚拟模块。解析导入时先查找项目、目标 triplet 专用目录和通用目录，文件均未命中时才使用内置声明。虚拟模块不生成目标文件或 `.sym`，也可通过 `export import` 转导出。调用解析到编译器提供的符号后，才按对应 builtin 生成表达式。

### 声明与泛型缓存格式

`.sym` 和泛型 `.mg` 使用带格式版本头的 MessagePack，结构变更时必须更新格式版本。
现有旧 TOML 构建缓存需重新生成；Project.toml 和用户配置仍使用 TOML。
