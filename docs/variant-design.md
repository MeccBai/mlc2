# Union / Variant

状态：首版已实现。Union 是封闭候选集合，不提供 any、继承或无标签 C union。

## 声明与构造

```rust
unit<T> Success { pub value:T; }
unit<E> Failed { pub error:E; }
union<T,E> Result[Success<T>,Failed<E>];

var x = Result<i32,i32>{Success<i32>{42}};
```

泛型参数也可写在名称后，与现有 Unit 保持一致。不能同时使用两种位置。
构造必须提供一个值，其类型精确匹配某一候选；不通过隐式数值转换选择 tag。
候选类型不能重复，泛型实例化后再次检查。泛型实参目前显式提供。

## 借用式 match

```rust
match (y = @mut x) {
    Success<i32> => { y->value = 100; },
    Failed<i32> => { y->error = 1; }
}
```

只读入口为 `y = @x`。每个分支的 y 是对应候选的普通 Ref，而不是 Res。
必须穷举全部候选，不允许 `_`、重复候选或按值绑定。
被借用对象必须已有存储，不接受临时构造值；已有 Ref 先解引用再借用。
分支绑定仅在本分支有效，不允许移出资源、逃逸引用或替换其所借用的整个 Union。

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
