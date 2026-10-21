# 安装目录与打包

```text
MLC/
  mlc.exe
  LLVM-C.dll                   # Windows 编译器启动依赖（如动态链接）
  tools/                       # ld.lld.exe、lld-link.exe 等宿主机工具
  lib/
    universal/                 # 通用 .m2 源码
    <triplet>/                 # 目标专用 .m2 源码
  objects/
    universal/                 # Object=None 的 .sym 声明
    <triplet>/                 # .sym、.obj/.o、.mg 缓存或二进制分发
  runtimes/
    <triplet>/                 # CRT 启动对象、基础库等链接依赖
```

源码搜索顺序：导入文件所在目录 → 配置的目标专用 lib 目录（按配置顺序）
→ 对应的 universal 源码目录。任何位置找到源码，都走正常源码构建流程，
由缓存哈希决定是否需要重新编译。

只有找不到源码时，才查导入文件旁的 .sym → objects/<triplet> →
objects/universal。universal 中的二进制声明仍只允许 Object=None，
不能放入某个目标的对象文件。损坏的声明报错，不静默尝试另一份声明。

库源码的生成产物写入对应 objects/<当前目标 triplet>，包括 universal 源码。
显式 LibCacheDir 优先；写入失败仍报警并回退到项目的
build/objects/lib/<triplet>。项目本身的产物位置保持不变。
配置 LibDirs 指向源码库根目录，目标 triplet 仍由现有配置逻辑拼接；
它的兄弟 objects 目录用于预编译产物。

## 打包脚本

```powershell
pwsh -NoProfile -File scripts/copy_to.ps1 -Profile release -Destination F:\Develop\MLC
# 自定义工具和运行库来源：
pwsh -NoProfile -File scripts/copy_to.ps1 -Profile debug -Destination out/MLC `
  -ToolsSource F:\Toolchain\tools -RuntimesSource F:\Toolchain\runtimes
```

脚本构建 mlc、复制源码、复制 objects 预编译产物，将仓库 lib 下的 TOML
声明转成目标目录 objects 下的 .sym，不向安装目录 lib 复制 TOML 或缓存。
仓库中的 TOML 保留为声明编写输入。工具和运行库默认取仓库 tools/、runtimes/；
目录不存在会明确警告，脚本不会自动下载工具链或猜测系统 CRT 位置。

脚本可以更新既有目录，但不会删除其中的旧文件。建议版本分发使用空目录，
以免旧 lib 中遗留的源码优先于新的 objects。LLVM-C.dll 若不在构建输出旁，
需要自行提供；它不是目标运行库，不能仅移动到 runtimes 后期待 Windows 自动加载。

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
