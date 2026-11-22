# 工具链分发实现

安装目录通过 MLC 环境变量指定；未设置时使用编译器目录。安装不读取旧 lib/、objects/。

~~~text
MLC/
  mlc.exe
  mlc-build.exe
  mlc-fmt.exe
  mlc-lsp.exe
  LLVM-C.dll
  tools/                       # lld-link、ld.lld、llvm-ar
  runtimes/<triplet>/           # CRT 启动对象和原生链接库
  packages/
    packages.toml
    <name>/<version>/
      source/                    # 所有 profile 共用一份源码
      debug/static/              # 可选
      release/shared/            # 可选
  cache/artifacts/             # Source 包按链接方式生成的原生库
~~~

std 和 c_std 与普通包使用相同的注册表和版本目录。工具链默认使用与编译器版本相同
的标准包，普通源码无需添加 Requires。标准库和 C 声明的选择规则见 [CLI 与项目配置](cli.md)。

标准包 source 目录中，std 的入口为 std.m2，子模块直接位于入口所在目录。
c_std 是纯声明包，配置 DeclarationOnly=true，没有 .m2 入口，不生成 .lib/.a 或 DLL。
例如 std::base 对应 std 的 source/base.m2，c_std::io 对应 c_std 的 source/io.h。
平台声明放在 source/targets/<triplet>/，同名输入覆盖通用输入。

运行 mlc setup 先转换头文件和 TOML 声明，再编译所有源码模块，包含未被使用的模块：

~~~powershell
mlc setup
mlc setup --release --triplet x86_64-pc-windows-msvc
~~~

声明位于 source/.compiled/declarations/<triplet>/<module>.sym，标准库对象位于
source/.compiled/objects/<profile>/<triplet>/<module>.obj（其他目标可能为 .o）。源码只保留一份，编译缓存按 debug/release 区分。项目对象仍在 build/objects/<profile>/<triplet>/，
IR 在 build/ll/<profile>/<triplet>/，可执行文件及 PDB 在 build/<profile>/。
setup 不调用链接器或归档器；源码包安装则必须先成功编译、归档或链接。

使用部署脚本构建并安装到 $env:MLC：

~~~powershell
$env:MLC = 'D:/mlc'
./scripts/deploy.ps1 -Profile debug
./scripts/deploy.ps1 -Profile release
# 使用已有构建产物，不重新调用 Cargo：
./scripts/deploy.ps1 -Profile release -SkipBuild
~~~

脚本复制所选 profile 的四个 EXE、对应 PDB（若生成）、LLVM-C.dll、原生工具和
runtimes，并部署一份各 profile 共用的标准包源码。生成缓存 .compiled 不从仓库复制；
安装后自动运行 mlc setup。仓库提供的同名同版本包记录会更新，其他已安装包记录和
文件保留。注册表与安装锁都位于 packages/ 下。LLVM DLL 从构建输出或当前 LLVM
开发环境取得；缺少程序、LLVM DLL 或必要的 MSVC 工具/运行库时，复制前直接报错。

mlc-build 和 mlc-lsp 加载 LLVM-C.dll；分发器与格式化器不加载 LLVM。
VS Code 扩展通过 MLC 定位 mlc-lsp。四个 EXE 应保持同目录。

Windows MSVC 目标调用 tools/lld-link.exe，GNU 目标调用 tools/ld.lld.exe。
runtimes/<triplet>/ 保存启动对象、.lib 或 .a；System32 中的 DLL 不能当作 .lib 链接。
MSVC 使用 msvcrt.lib、vcruntime.lib、ucrt.lib、kernel32.lib、
legacy_stdio_definitions.lib、oldnames.lib。工具与运行库不属于语言标准包。
