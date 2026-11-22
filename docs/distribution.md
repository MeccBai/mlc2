# 安装与分发

将工具链解压到同一目录，并将该目录加入 PATH。可以用 MLC 环境变量指定工具链目录。未设置时，使用编译器所在目录。VS Code 扩展通过 MLC 查找语言服务器。

运行以下命令准备标准包。需要为指定目标准备 release 版本时，传入对应选项。

~~~powershell
mlc setup
mlc setup --release --triplet x86_64-pc-windows-msvc
~~~

工具链默认使用与编译器版本相同的 std 和 c_std，普通项目可直接导入。Windows 的不同目标需要对应的链接工具和运行库。可用 --runtime-dir 指定运行库路径，目标选择和链接参数见 [CLI](cli.md)。

普通库使用 mlc install <target> 安装，Requires 精确指定本地包版本与表示方式，
详见 [CLI](cli.md)。Source 可静态或动态链接，Static/Shared 的链接方式固定。
