# 构建流水线（当前实现）

实际 CLI 编排已接入，命令及当前链接工具约束见 [CLI](cli.md)。
本文主要描述可复用的低层构建接口；CLI 的 Package 分析在主线程完成。

## 安装布局与查找顺序

```text
installation/
  mlc.exe
  lld-link.exe / ld.lld.exe
  LLVM-C.dll 和其他运行时 DLL
  lib/
    <triplet>/
      std/
        io.m2 或 io.sym
```

`BackendConfig.tools` 指向安装根目录，不再指向 `tools/` 子目录。
`ImportResolver::from_executable()` 默认使用安装根目录的 `lib/<host-triplet>/`；
调用方可以通过 `lib_dirs` 增加有序库路径。

`import std::io;` 查找顺序为：

1. 当前导入文件所在目录的 `std/io.m2`、`std/io.sym`。
2. 各系统库目录的 `std/io.m2`、`std/io.sym`。

目录优先级高于文件格式优先级：本地 `.sym` 优先于系统 `.m2`。
所有存在的文件路径规范化后去重；重复 import 只产生一条依赖边。
递归扫描会在启动线程池前发现并返回依赖环。

## 三层接口

| 层 | 入口 | 职责 |
| --- | --- | --- |
| 持久化 | `mlc_syntax::serialization`、`mlc_builder::artifacts` | Temp TOML、元数据、内容哈希、发布与校验 |
| 构建计划 | `BuildPlan::discover(entry, resolver)` | 文件发现、索引图、反向依赖、环检测、依赖优先顺序 |
| 调度执行 | `schedule::build(plan, options, compile)` | taskflowrs 依赖任务、缓存决策、失败阻断、LLVM object 输出 |

`compile` 接收 `CompileRequest`，包含当前源码节点、直接依赖的
`Artifact` 和构建选项。成功返回 `CompileOutput { ir: Some(ir) }`，
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

通过 `PathResolver` 构建时，项目模块 `std::io` 位于
`build/objects/<triplet>/std/io.*`；库默认在 `lib/<triplet>/std/io.*` 原地缓存。
显式库缓存目录或原目录写入失败时，回退到
`build/objects/lib/<triplet>/std/io.*`，并收集警告。
低层 `BuildPlan::discover()` 仍保留调用方直接指定输出目录的接口。

- `.sym`：MessagePack 二进制，保存版本、模块名、源文件路径、目标、构建键、依赖指纹和声明。
- `.obj` / `.o`：LLVM 输出，根据 target 选择后缀。
- `.mg`：存在泛型模板时保存 TempModule 的 MessagePack 二进制表示。

`.mg` 暂时保存整个翻译单元，包含私有 helper、using、全局变量和函数体，
以保留模板的依赖闭包。普通函数声明只保存 symbol，不在声明表内保存 body。
inner 声明不等于对外可见声明，不能直接拼接进公共搜索空间。

序列化保留 Span 和现有类型字段，不持久化运行期 arena index 或 Rc。
二进制文件带有 magic 和版本头；旧格式构建缓存需要重新生成。
手写 C 声明仍可维护为 TOML，通过 `mlc symbols io.toml` 生成 `io.sym` 后导入。
普通 import 不再直接读取 TOML；Project.toml 和全局配置格式不变。
TOML 没有 null：编译器生成格式用保留表 `{ "$none" = true }`
表示 `None` 和 unit 值；因此这不是最终面向人工编写的精简 C FFI 格式。

缓存键包含源文件哈希、目标 triplet、调用方提供的 compiler_id、
compiler_options 和依赖指纹。调用方必须让 compiler_id 对编译器语义变化
敏感，并在 compiler_options 中完整记录影响编译的设置。

普通函数体变化只使自身重编译。公开声明或泛型闭包变化会向使用者传播。
目前哈希基于 Temp 声明而不是解析后的类型布局，所以声明哈希保守地包含
依赖指纹，避免间接布局变化漏编译；这可能产生额外的传递性重编译。
Span 不参与语义哈希，但源文件任何变化都会改变自己的 FileHash。

产物格式版本为 2，`Object` 表示 `Source`、`None` 或 `Only` 模式，
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
所有 triplet 库目录都未找到模块时，再依次搜索对应库根目录的
`universal/<module>.sym`。该回退仅接受 `Object = "None"`，不读取源码。
仓库提供 `lib/universal/c_std/io.sym` 中的 C ABI `printf` 声明，
通过 `import c_std::io;` 使用；实际实现和链接库由目标 C 运行时提供。
项目配置的相对路径基于项目根目录，全局配置的相对路径基于配置文件目录。

全局配置示例：

```toml
LibDirs = ["D:/mlc/lib", "D:/extra/lib"]
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

同样，缓存中的文件身份需要稳定分配；示例测试使用 TargetIndex 仅适用于
固定的小型测试图。正式入口需要稳定的文件身份清单，或去除生成符号中
对临时文件编号的依赖，避免图变化后复用缓存造成全局初始化函数重名。

## 验证

```text
cargo test --workspace
cargo test --test hello_world -- --nocapture
cargo test --manifest-path experiments/taskflow/Cargo.toml
```

覆盖 Temp 往返、源码到 object、增量缓存、损坏恢复、模板变化、
传递依赖变化、独立任务失败隔离、目录优先级、重复依赖和环检测。

Windows 的 `hello_world` 测试加载仓库 `lib/universal/c_std/io.sym`，
编译 `tests/fixtures/hello_world.m2`：构造 i8 数组、手动将最后一项写为 0，
将第一项的引用传给 printf。MLC/LLVM 生成 object 后，使用 PATH 中的
LLVM-MinGW/UCRT `clang -fuse-ld=lld` 链接（可用 `MLC_TEST_CLANG` 指定驱动）。
运行结果必须为退出码 0、stdout `Hello World!\r\n`、stderr 为空。
该测试不跳过缺失工具，也不替代正式的入口/链接参数编排。
