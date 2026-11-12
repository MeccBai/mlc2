# 安装目录与打包

```text
MLC/
  mlc.exe
  mlc-build.exe                 # 编译/构建工作进程
  mlc-fmt.exe                   # 格式化工作进程
  mlc-lsp.exe                   # 保存后分析的独立 stdio LSP
  LLVM-C.dll                    # mlc-build 和 mlc-lsp 的启动依赖
  tools/                       # ld.lld.exe、lld-link.exe 等宿主机工具
  lib/
    universal/                 # 通用 .m2 源码
    <triplet>/                 # 目标专用 .m2 源码
  objects/
    universal/                 # Object=None 的 .sym 声明
    debug/<triplet>/           # debug 标准库 .sym、.obj/.o、.mg
    release/<triplet>/         # release 标准库 .sym、.obj/.o、.mg
  runtimes/
    <triplet>/                 # CRT 启动对象、基础库等链接依赖
```

`mlc` 只解析参数并分发，`fmt` 启动同目录的 `mlc-fmt`，其他业务命令启动
`mlc-build`。帮助和版本在分发器内直接完成。构建工作进程和 LSP 导入 LLVM-C.dll；
分发器和格式化器不加载 LLVM。四个程序保持同目录，工作进程继承当前目录、
标准输入输出和环境，退出码回传。开发构建请使用 `cargo build --bins`。
编辑器直接启动 `mlc-lsp`，不经过分发器；用法见 [LSP](lsp.md)。

源码搜索顺序：导入文件所在目录 → 配置的目标专用 lib 目录（按配置顺序）
→ 对应的 universal 源码目录。任何位置找到源码，都走正常源码构建流程，
由缓存哈希决定是否需要重新编译。

只有找不到源码时，才查导入文件旁的 .sym → objects/<mode>/<triplet> → 兼容的 objects/<triplet> →
objects/universal。universal 中的二进制声明仍只允许 Object=None，
不能放入某个目标的对象文件。损坏的声明报错，不静默尝试另一份声明。

库源码的生成产物写入对应 objects/<mode>/<当前目标 triplet>，包括 universal 源码。标准库仅使用 debug/release 两档；default/debug 默认选择 debug，release/rel-with-info 默认选择 release。
显式 LibCacheDir 优先；写入失败仍报警并回退到项目的
build/objects/<mode>/<triplet>/lib。项目对象放在 build/objects/<mode>/<triplet>，IR 放在 build/ll/<mode>/<triplet>，最终产物放在 build/<mode>。
配置 LibDirs 指向源码库根目录，目标 triplet 仍由现有配置逻辑拼接；
它的兄弟 objects 目录用于预编译产物。

## 打包脚本

```powershell
pwsh -NoProfile -File scripts/copy_to.ps1 -Profile release -Destination F:\Develop\MLC
# 生成供安装器封装的完整目录，缺少默认 MSVC 工具/运行库时直接失败：
pwsh -NoProfile -File scripts/copy_to.ps1 -Profile release -Destination target/package -RequireComplete
# 自定义工具和运行库来源：
pwsh -NoProfile -File scripts/copy_to.ps1 -Profile debug -Destination out/MLC `
  -ToolsSource F:\Toolchain\tools -RuntimesSource F:\Toolchain\runtimes
```

脚本构建 mlc、复制源码、复制 objects 预编译产物，将仓库 lib 下的 TOML 或 .h
声明转成目标目录 objects 下的 .sym，不向安装目录 lib 复制声明输入或缓存。
头文件位于 lib/<triplet>/<module>.h，打包时按路径指定模块名和目标。
仓库中的 TOML 和 .h 保留为声明编写输入。工具和运行库默认取仓库 tools/、runtimes/；
目录不存在会明确警告，脚本不会自动下载工具链或猜测系统 CRT 位置。

脚本可以更新既有目录，但不会删除其中的旧文件。建议版本分发使用空目录，
以免旧 lib 中遗留的源码优先于新的 objects。LLVM-C.dll 按显式 `-LlvmDllSource`、
构建输出、SCOOP 的 llvm-dev/bin、PATH、已有目标安装目录的顺序寻找；找不到会终止。
DLL 必须与构建时链接的 LLVM-C.lib 匹配。它不是目标运行库，不能仅移动到 runtimes。
`-RequireComplete` 检查 lld-link.exe、llvm-ar.exe 和默认 MSVC 的六个导入库；
不代表验证所有可选 GNU 组件、所有 DLL 的传递依赖或再分发许可。

## 一键安装 / MSI（方案，尚未实现）

保留两层流程：`copy_to.ps1` 生成目录包，独立安装项目将目录包封装为安装器。
不把安装、注册表或卸载逻辑塞入编译器。

需要 `.msi` 时可采用 WiX：

1. 从空的 staging 目录生成 release 包，带 `-RequireComplete`，不要打包开发机残留缓存。
2. 用 WiX 的 Files 递归收集目录，保留三个 EXE、LLVM DLL、tools/lib/objects/runtimes 的相对位置。
3. 定义安装目录、版本、稳定的 UpgradeCode、升级/卸载行为，以及可选 PATH 注册；
   仅追加 MLC 目录，卸载时只撤销自己添加的项，不覆盖整个 PATH。
4. 在干净 Windows 环境验证安装、普通用户编译、升级、卸载及 DLL 依赖。
   检查 VC Runtime 和 LLVM 所需运行时，不以开发机安装状态代替依赖验证。
5. 发布前处理各工具和运行库所需的许可文件，并对安装包签名。

若装入 Program Files，标准库缓存不能假设可写；使用用户级 LibCacheDir 或现有项目缓存回退，
不要要求每次编译提权。安装器也不应修改或删除用户项目的 build 目录。

WiX 可使用 .NET 工具或 MSBuild；生成后的 MSI 支持 Windows Installer 安装/卸载及静默模式。
若只需要双击安装 EXE 而非 MSI，也可考虑 Inno Setup。当前脚本只生成目录包，不会生成 MSI、
修改 PATH 或安装额外工具。

参考：[WiX](https://docs.firegiant.com/wix/using-wix/)、
[Files 目录收集](https://docs.firegiant.com/wix/schema/wxs/files/)、
[Environment / PATH](https://docs.firegiant.com/wix/schema/wxs/environment/)、
[msiexec 参数](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options)。

## Windows linker profiles

Windows 默认目标为 `x86_64-pc-windows-msvc`。`src/driver/link/targets.rs`
按 triplet 选择工具和参数，未配置的目标会明确报错。

| Triplet | Linker | Runtime libraries |
| --- | --- | --- |
| x86_64-pc-windows-msvc | tools/lld-link.exe | runtimes/x86_64-pc-windows-msvc/*.lib |
| x86_64-pc-windows-gnu | tools/ld.lld.exe | runtimes/x86_64-pc-windows-gnu 的 CRT 对象和 .a |

MSVC 配置使用 msvcrt.lib、vcruntime.lib、ucrt.lib、kernel32.lib、
legacy_stdio_definitions.lib 和 oldnames.lib；EXE 注入 /subsystem:console，DLL 注入 /dll。
完整的随包运行库优先使用，不依赖系统 SDK。缺库时可从开发环境 LIB、
VCToolsInstallDir、vswhere 发现的 Build Tools 和 Windows SDK 查找。
System32 中的 DLL 是运行时实现，不作为 .lib 链接输入。

完整链接验证：

```powershell
cargo test --test msvc_cli -- --include-ignored
cargo test --test stdlib_cli -- --include-ignored
```

`lib/universal/c_std/*.h` 按 `--triplet universal` 转换到 `objects/universal/c_std/*.sym`；`lib/<triplet>/c_std/*.h` 使用对应 triplet。通用头文件只包含已支持 64 位平台上 ABI 一致的接口。

Run `mlc setup` after copying library sources into an installation. It converts all declaration inputs and prepares both debug/release object variants for all target directories, without requiring the native linker. Use `mlc setup --triplet <triplet> --release` to prepare only one target and variant. Source declarations remain in neutral `objects/universal` or target-specific `objects/<triplet>`; compiled sources are cached under `objects/<mode>/<triplet>`.
