# Union / Variant 实现

使用方法见 [Union 语法](language.md#15-union--variant)。

## 复用与 lowering

声明容器、泛型、符号注册、interface、导入导出复用 Unit 的基础设施，
通过独立 UnionType 元数据保留候选及 tag 语义，不向语言暴露合成字段。
Union 可定义 interface；不允许 C ABI。

LLVM 布局为 i32 tag + 最大候选尺寸的内联 payload，按最大候选对齐并保留 padding。
运行时只存储一个候选，不同时分配全部候选。match 降为 tag switch，
分支引用指向同一 payload 存储，但拥有具体的静态类型。

任一候选含 Res 时，Union 自动成为 move-only；析构根据当前 tag 递归清理活跃候选，
不清理未激活的候选。整体 move、覆盖赋值、函数传参和返回复用现有 RAII 通路。
析构计划在具体实例解析时生成，生成端按符号去重输出。
