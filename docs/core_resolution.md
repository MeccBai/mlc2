# Core 跨文件解析

`PackageSymbolTable` 管理全部文件 arena、Config 和导出索引。
`ResolveScope` 只管理名字查询权限：当前文件和直接导入的文件 id。
`TypeContext` 保持原有职责，仅管理当前声明或函数的泛型绑定。

## 入口

调用方完成文件发现和 id 分配，依赖优先准备其声明、模板和必要的全局变量：

```rust
dependency.export(&mut package);
dependency.analysis(&mut package);

package.set_imports(
    entry.config.file_id(),
    [dependency.config.file_id()],
);
entry.analysis(&mut package);
```

AST 的声明解析和 body 解析均自动使用 package 的查询范围。
未设置 imports 时，默认只有当前文件，不会因为某个文件已经加载而自动开放。
这里不负责从文件系统加载 import；构建编排器负责路径到 FileId 的映射。

直接解析一个类型时：

```rust
let mut symbols = package.resolve_context(config.file_id());
let ty = resolve_type(&mut config, temp_type, &mut symbols, type_context);
```

解析接口接受 `Resolution`，既可以使用这个 package 视图，也兼容原来的
单文件 `SymbolTable`。现有 AST 类型结构和 index 字段没有改变。

## 搜索与取数据

- 泛型绑定仍优先于普通类型名字。
- 当前文件声明可访问私有类型、函数、接口及 using。
- 外部名字只能来自 imports 范围内的 searchable。
- inner 只对当前定义文件开放，不能搜索被导入文件的 inner。
- using 优先级最低，展开后的目标仍遵守相同范围。
- 表达式继续沿用已有 path 优先级，包括枚举值高于全局变量。

`lookup_in(name, scope)` 用于受限名字搜索；旧的 `lookup(name, caller)`
保留为 Package 的无 import 范围检查查询接口，解析流程不使用它绕过范围。

已经解析的 TypeIndex、FuncIndex、InterfaceIndex 按 file id 访问原始 arena，
不再通过当前文件 arena 读取外部 index。`TypeLookup` 用于只读类型访问，
`TypeStorage` 将新的引用、数组、值限定类型存入当前活动 arena。

名字权限不会截断已知类型的内部依赖。例如公开函数可以返回一个不可直接
搜索的私有类型，类型检查仍可访问这个 index；其成员访问仍检查 pub。
对已知值调用关联接口同样属于类型关联操作，不要求泛型定义文件预先 import
未来调用方的类型模块，但仍检查 public/self 规则。直接书写模块符号路径
则仍需满足 import 范围。

限定值类型的缓存不会把外部私有类型的原名暴露到当前文件搜索空间，
即使之后重新发布当前文件的 inner 表，也不会变成可搜索声明。

## 泛型实例化

外部模板只有 exported 才能直接实例化；私有模板只能由同文件发起。
开始实例化后，使用定义文件的 Config、名字空间和 import 范围解析参数、
类型与 body，因此导出模板可以使用自己的私有 helper 和内部类型。

实例及 body 存入定义文件的原始 arena，多个调用方按相同名字和类型参数
复用同一个实例。递归实例化仍共用 actives 表。符号参数继续保留模板身份，
直到具体参数可用时再实例化。

返回调用方时恢复查询范围。外部模板产生错误时，将错误归类保留并定位到
调用方的实例化 Span，避免把另一份源码的 Span 交给调用方打印器。
失败实例撤销名字缓存，不会让后续调用拿到未完成的 symbol；已分配的
arena slot 不移动，以保证现存 index 不发生错位。

## 生成阶段

定义文件完成 analysis 后仍可能收到其他文件触发的新实例，所以其 AST.body
不是实例的实时总表。编排器应从 Package 中各文件的 `function_instances`
和 `interface_instances` 收集新 body，交给 generator 的去重机制处理。
`tests/build_pipeline.rs` 包含跨文件 AST、泛型 body 到 LLVM object 的验证。
