# ffi_parser

受限 C 声明输入 → `mlc_syntax::TempModule`，不依赖 core 或 LLVM，不调用预处理器。
`parse(source, target)` 返回声明或带字节 span 的错误。builder 负责生成 Object=None
的目标专用 `.sym`；普通 MLC import 不直接解析头文件。

```c
struct Point {
    int32_t x;
    int32_t y;
};
void update(struct Point *point);
int32_t print(const int8_t *pattern, ...);
```

```powershell
mlc symbols native.h                         # native.sym，模块名 native，宿主 triplet
mlc symbols native.h --module c_std::native --triplet x86_64-pc-windows-msvc -o native.sym
```

仅支持完整具名 struct 和函数原型。类型名字为编译器内建识别，不需要也不允许
`#include`：int8_t/int16_t/int32_t/int64_t、对应 uint 类型、float、double、bool/_Bool、
void、指针、struct 成员定长数组。允许 const 修饰指针指向的内容、未命名函数参数、
末尾可变参数（至少一个固定参数）。无参数函数必须写 `(void)`，而不是旧式 `()`。

拒绝 int/char/short/long/signed/unsigned/size_t 等不明确宽度的名字，以及所有
预处理（含头文件保护）、typedef、enum、union、全局变量、函数体、函数指针、
inline/static/extern、属性、volatile/restrict、指针本身的 const、const 值成员、
位域、柔性数组、匿名/空/不完整 struct、多声明符。struct 按值成员允许，函数按值
接收或返回 struct 暂不支持，必须通过指针。引用的 struct 必须在同一文件完整定义，
支持完整定义间的相互指针引用；拒绝递归值布局、重复名字和布局溢出。

当前接受 x86_64/aarch64 的 Windows/Linux/macOS 目标；拒绝 32 位目标，避免与
现有语义层固定 64 位引用大小不一致。产物记录 triplet，不能跨目标误用。
这不是系统头文件兼容层，也不会静默忽略不支持的 C 语法。
当前没有符号重命名机制，因此导出的 C 名字和成员名不能与 MLC 关键字冲突。

共享声明可用 `--triplet universal` 转换，生成的 Object=None 符号表不绑定具体 triplet。仅适用于所有已支持 64 位目标上 ABI 一致的显式宽度接口；不会推断 size_t、long、wchar_t、time_t 或系统结构布局。类型限制仍然适用。有 ABI 差异的接口继续使用目标专用头文件，不能因其属于 C 标准就视为 universal。
