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
```

Type 仅为 bin/static/shared。project target 的 Triplet、LibDirs、LibCacheDir
覆盖项目与全局配置；CLI 的 --triplet、--lib-dir、-o 优先于配置。
可用 --config 指定全局配置文件；-j/--jobs 控制 LLVM object 任务数量。

## 当前链接工具

编译使用 MLC 前端与 llvm-sys，不调用 clang 编译源文件。
链接使用 `clang -fuse-ld=lld` 驱动，由它配置 CRT/SDK、启动代码和库路径；
通过 --linker 指定驱动、--link-arg 添加参数。静态库使用 llvm-ar，可用
--archiver 指定。两者均通过进程参数数组调用，不经过 shell。

当前实际验证环境是 Windows x64 + LLVM-MinGW/UCRT；默认 MSVC triplet
生成的标量/指针 C ABI object 已验证可由该驱动链接。不代表不同工具链的
所有聚合 C ABI 均已兼容。其他 triplet 显式传入驱动，需要相应 sysroot/运行时。
bin 入口要求 main 无参数，返回 i32 或 void。
shared 可生成动态库；带全局运行期初始化的 static 暂时报不支持，避免漏初始化。
链接/归档先写临时文件，成功才替换最终产物，避免旧 archive 残留成员。

## 编排和缓存边界

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
cargo test --test hello_world
```

测试实际构建、链接并执行，验证 stdout、退出码和 object 缓存复用。
