# MLC 语言语法与示例

本文描述当前编译器已经支持的源码语法，不把设计草案当作已实现功能。
源码后缀当前为 `.m2`，属于可调整的编译器常量。
标为 `rust` 的代码块是可独立检查的完整模块；标为 `text` 的代码块是语法片段、跨文件片段或故意错误的示例。
构建与 CLI 参数见 [CLI 文档](cli.md)，构建缓存与产物见 [构建管线](build_pipeline.md)。

## 1. 最小程序

入口通常为无参数、返回 i32 的 main，由构建端生成入口适配。
普通 main 不需要为了成为入口而加 C ABI attribute。

```rust
import c_std::io;

func main() -> i32 {
    c_std::io::printf(const_c_str("Hello World!\n"));
    return 0;
}
```

这里的 c_std::io 是已提供的 C 声明模块，不是完整的 MLC 标准库。
可以用 `mlc main.m2` 构建，或 `mlc ll main.m2` 只生成 LLVM IR。

## 2. 词法与标点

标识符匹配 `[a-zA-Z_][a-zA-Z0-9_]*`，区分大小写。
空格、Tab、LF、CRLF 均可分隔 token；支持单行注释与非嵌套块注释。
声明与普通语句通常以分号结束。unit、enum、generic 的右花括号后分号可省略；
union、import、using、变量声明必须带分号。控制流及函数体不需要末尾分号。

```rust
// 单行注释
/* 块注释 */
func main() -> i32 {
    var number = 123;        // 默认 i32
    var fraction = 1.5;      // 默认 f64
    var scientific = 1.0e3;
    var enabled = true;
    return number;
}
```

整数目前只支持十进制字面量；负数由一元 - 构成。
浮点支持小数和指数形式，例如 1.5、.5、1.、1e3。
不提供字符字面量、十六进制整数、数字下划线或字面量类型后缀。
字符串为双引号 UTF-8 文本，支持 `\n`、`\r`、`\t`、`\0`、`\\`、`\"`；
未知转义报错。字节容量按解码后的 UTF-8 长度计算，而不是字符数。

保留关键字：union、func、var、val、const、mut、res、unit、using、generic、enum、
import、export、pub、global、api、in、true、false、if、else、while、for、break、
continue、match、return、null、anonymous；`_` 是 match 默认分支标记。
null 已有词法与临时节点，但尚无完整的公共空指针初始化约定，不应视为通用可用的指针值。

嵌套泛型闭合时注意连续 `>>` 会被识别为右移，必要时写作 `> >`；
泛型类型紧接 `=>` 时写 `Type<T> =>`，避免 `>=` 合并。

## 3. 类型

类型后置：变量、字段及参数写成 `name:Type`，函数返回类型写成 `-> Type`。

| 类型形式 | 含义 | 示例 |
|---|---|---|
| i8、i16、i32、i64 | 有符号整数 | i32 |
| u8、u16、u32、u64 | 无符号整数 | u32 |
| f32、f64 | 浮点数 | f64 |
| bool | 布尔值 | bool |
| Name / module::Name | 命名类型 | Point |
| Name<T,...> | 泛型类型的具体应用 | Box<i32> |
| [T:N] | 固定长度数组 | [i32:10] |
| $T | 普通只读引用 | $i32 |
| $mut T | 普通可变引用 | $mut i32 |
| res $T / res $mut T | 拥有内存资源的引用 | res $mut i32 |
| func(T1,T2)->R | 函数指针 | func(i32)->i32 |
| func(T1,T2) | 无返回值函数指针 | func(i32) |

类型形式可以递归组合，如 `[func(i32)->i32:2]`、`$[i32:10]`。
数组长度必须是可解析为 usize 的整数字面量，不是任意常量表达式。
省略函数返回类型表示不返回值，不需要写 void。
函数返回值的顶层可变性为可变；引用所指对象的可变性仍由 $ / $mut 决定。

## 4. 变量、常量与赋值

| 声明 | 语义 |
|---|---|
| var name[:Type] = value; | 可修改的变量 |
| val name[:Type] = value; | 初始化后不可重新赋值 |
| const name[:Type] = value; | 必须由编译期常量初始化 |
| global var / val / const ... | 当前模块的全局变量 |

全部变量必须初始化。省略类型时由右侧推断。
可变性存储在类型本身，不是 TypeIndex 上的独立标志。
val 引用自身不可重绑，但若其 pointee 为可变，仍可通过引用修改 pointee。
const 不是任意函数的编译期执行；普通用户函数调用不能作为 const 初始化器。

```rust
global var counter:i32 = 0;
global const limit:i32 = 10;

func main() -> i32 {
    var value = 1;
    val fixed:i32 = 2;
    const folded = (2 + 3) * 4;
    value = fixed + folded;
    counter = counter + 1;
    return value;
}
```

赋值左侧可以是可变变量、字段、索引或解引用，不限于变量名称。
不支持变量遮蔽：局部变量不得覆盖当前可见的变量、参数、泛型参数或 self。
同一模块的命名符号不允许重名，因此也不提供同名函数重载。
全局变量不能 export / api，也不支持 module::variable 跨模块访问。
全局初始化由生成的初始化函数执行；库初始化具有重复调用保护并初始化依赖。

### 数值赋值与显式转换

初始化允许无损数值转换；可求值常量根据目标范围和精度检查。
运行期转换只有在源类型全部取值均可无损表示时才隐式允许。
普通函数实参仍遵循参数类型检查，不能假设初始化宽容适用于所有调用。

```rust
func main() -> i32 {
    var positive:u32 = 10;
    var exact:f32 = 1.5;
    var small:i8 = 12;
    var wider:i32 = small;
    var deliberate:i8 = cast<i8>(300);
    return wider;
}
```

`var x:i8 = 300;`、`var x:u32 = -1;`、`var x:f32 = 0.1;` 会报范围或精度错误。
cast<T>(value) 表示主动转换，不按目标数值范围拒绝输入；它不是饱和转换。
当前内建 cast 支持基本类型间转换，以及 Ref 到 Ref 的转换，不能替代任意聚合类型转换。
没有 i8(300) 这类类型名调用式转换；唯一内建转换入口是 cast。

## 5. 表达式与运算符

| 优先级：高 → 低 | 语法 |
|---|---|
| 分组、后缀访问 | (expr)、call(...)、expr.member、expr->member、expr[index] |
| 前缀 | -expr、!expr、~expr、@expr、@mut expr、$expr |
| 乘除余 | *、/、% |
| 加减 | +、- |
| 移位 | <<、>> |
| 大小比较 | <、<=、>、>= |
| 相等比较 | ==、!= |
| 按位与 | & |
| 按位异或 | ^ |
| 按位或 | \| |
| 短路与 | && |
| 短路或 | \|\| |
| 管道 | \|> |

同级二元运算左结合，括号改变结合次序。比较返回 bool，&& / || 保持短路求值。
& 是按位与，不是取引用；* 是乘法，不是解引用。
不同运算符仍要求合法操作数类型，不因共享表格就允许任意类型运算。
赋值是语句，不是可嵌入其他表达式的赋值表达式；不提供 ++、--、+= 或三元 ?:。

```rust
func main() -> i32 {
    var a = 5;
    var b = 3;
    var arithmetic = a + b * 2;
    var grouped = (a + b) * 2;
    var remainder = arithmetic % 4;
    var bits = ((a << 1) | b) ^ 1;
    var condition = a > b && b != 0;
    if (condition) { return grouped + remainder; }
    return bits;
}
```

## 6. 数组与字符串

数组初始化固定使用 []，不使用 {}。
显式容量的数组允许少填元素：给出 warning，未填部分清零；多填报错。
这是初始化规则，不代表不同长度数组可以任意互相赋值。

```rust
func main() -> i32 {
    var inferred = [1,2,3];
    var padded:[i32:5] = [1,2]; // warning，后面三个元素补 0
    var empty:[i32:4] = [];    // warning，全部补 0
    var nested:[[i32:2]:2] = [[1,2],[3,4]];
    var text:[i8:20] = "hello world";
    text[19] = 0;
    inferred[1] = nested[0][1];
    return inferred[1] + padded[4] + empty[0];
}
```

字符串可以初始化容量足够的 i8 字节数组；多余空间清零。
普通字符串字面量不承诺自动追加 C 的结束零；需要 C 字符串时使用 const_c_str，
或显式保留额外容量并写零。不提供独立的字符串对象、切片或动态数组容器语法。

## 7. Ref、取引用、解引用与索引

```rust
func main() -> i32 {
    var value = 10;
    var readonly:$i32 = @value;
    var writable:$mut i32 = @mut value;
    $writable = 20;

    var array = [1,2,3];
    var array_ref:$mut [i32:3] = @mut array;
    ($array_ref)[1] = 9;
    return $readonly;
}
```

@ 默认创建只读 Ref，@mut 显式创建可变 Ref；val / const 值不能获得可变借用。
普通 Ref 不拥有内存，也不会析构其指向内容。
引用可以继续取引用，$$T 表示更深层级。
索引接受数组与整型索引；Ref 的索引访问已开放，可用于指针指向的动态存储。
普通 Ref 的索引不携带容量，不提供自动长度或完整边界安全保证。
取引用和解引用的组合注意优先级，复杂表达式可以明确加括号。

## 8. Unit、初始化列表与字段

```rust
unit Point {
    pub x:i32;
    pub y:i32;
}

func main() -> i32 {
    var point = Point{1,2};
    var inferred:Point = {3,4};
    point.x = inferred.y;
    var reference = @point;
    return reference->x;
}
```

字段按声明顺序初始化，可递归使用 Unit{...} 和数组初始化。
这里的 {} 是聚合初始化列表，不是数组字面量、tuple 或匿名作用域。
无显式类型的 {...} 需要预期类型，或作为管道参数组使用。
点号用于本体字段访问，箭头用于引用字段访问。
self 的访问必须使用 ->，不能使用 .；self 可访问私有字段，其余访问检查字段 pub。
不提供按字段名初始化或字段默认值语法。

## 9. 普通函数、返回与可变参数

```rust
func add(a:i32,b:i32) -> i32 {
    return a + b;
}
func notify(value:i32) {
    return;
}
func main() -> i32 {
    notify(1);
    return add(2,3);
}
```

参数都需要声明类型；有返回类型的 return 必须携带相符值，
无返回类型只能 return;。源码普通函数必须有 body，不支持仅声明的 func f(...);。
C 外部声明来自符号描述文件，不通过无 body 源码函数导入。
可变参数以末尾 ... 表达，例如 `func log(format:$i8,...){...}`；
... 后不能再声明参数。当前不提供源码侧的 va_list / va_arg 读取 API。

## 10. Interface 与 self

Interface 是绑定在 Unit / Union 上的可调用接口，不是 Rust trait。
有 self 时调用传入实例；无 self 时为类型关联调用。泛型列表位于 func 后、名称前。

```rust
unit Counter { value:i32; }

pub Counter::func read(self) -> i32 {
    return self->value;
}
pub Counter::func inspect_mut(mut self) -> i32 { return self->value; }
pub Counter::func create(value:i32) -> Counter {
    return Counter{value};
}
func main() -> i32 {
    var counter = Counter::create(1);
    var inspected = counter.inspect_mut();
    return counter.read();
}
```

self / mut self 只能位于第一个参数位置。
只读 self 不允许修改本体，mut self 要求调用对象允许可变访问。
当前实现缺口：mut self 的字段写入（self->field = value）仍被左值检查误拒绝；
语法与可变调用规则已接入，但本节不把成员写入列为已经验证可用的能力。
Interface 可以带普通参数、泛型和返回类型；没有继承或虚方法分派。
成员调用不允许作为管道目标，std::function 也不支持取得 Interface 的地址。

## 11. 可见性

| 声明 | 默认 | pub | export | api |
|---|---|---|---|---|
| Interface | 私有接口 | 单元外可调用 | 模块导出，单元访问仍受限 | 同时 pub 与 export |
| func、unit、union、enum、generic、using、import | 模块内部 | 非法 | 模块导出 / 转导出 | 非法 |
| Unit 字段 | 私有 | 允许外部访问 | 非法 | 非法 |
| global 变量 | 当前模块内部 | 非法 | 非法 | 非法 |

pub 不等于 export；api 是 Interface 的两种可见性同时开启。
仅支持一个可见性关键字，不写 pub export 组合。
Interface 的私有成员访问使用 self；外部对象访问不能忽略 pub。

```rust
export unit Device { value:i32; }
api Device::func read(self) -> i32 { return self->value; }
export func answer() -> i32 { return 42; }
func main() -> i32 {
    var device = Device{answer()};
    return device.read();
}
```

## 12. 管道

管道是普通调用的语法糖，不创建独立的运行时对象。
单值置于显式参数之前；无类型 {...} 会拆成多个调用参数。可以连续连接。

```rust
func add(a:i32,b:i32) -> i32 { return a + b; }
func twice(value:i32) -> i32 { return value * 2; }
func main() -> i32 {
    var a = 3 |> twice;
    var b = 3 |> add(4);
    var c = {2,5} |> add |> twice;
    return a + b + c;
}
```

分别等价于 twice(3)、add(3,4)、twice(add(2,5))。
目标语法为普通路径，可写 module::function；不支持 obj.method 目标。
当前管道没有独立的显式泛型实参语法，可由普通函数调用的泛型推断处理。

## 13. 泛型与约束

普通函数写 func<T,...> name，Unit / Union 支持 Name<T,...> 或 <T,...> Name 两种位置，
但不能同时在前后各声明一份泛型参数。约束为 T:Requirement，没有约束则为 T。

```rust
generic Integer {
    std::generic::is_integer;
}
func<T:Integer> swap(a:$mut T,b:$mut T) {
    var temporary = $a;
    $a = $b;
    $b = temporary;
}
unit<T> Box { pub value:T; }

func main() -> i32 {
    var a = 1;
    var b = 2;
    swap<i32>(@mut a,@mut b);
    swap(@mut a,@mut b);
    var boxed = Box<i32>{a};
    return boxed.value;
}
```

| 内置约束 | 含义 |
|---|---|
| std::generic::is_integer; | 整数类型 |
| std::generic::is_float; | 浮点类型 |
| std::generic::is_signed; | 有符号数值类型 |
| std::generic::min_bits<N>; | 最小位宽 |
| std::generic::max_bits<N>; | 最大位宽 |

这些约束名称由编译器识别，无需 import std::generic。
一个 generic 内各要求同时满足；也可要求具体 Interface 签名。

```rust
generic Readable {
    pub func read(self) -> i32;
}
unit Item { value:i32; }
pub Item::func read(self) -> i32 { return self->value; }
func<T:Readable> read_value(value:T) -> i32 { return value.read(); }
func main() -> i32 { return read_value(Item{42}); }
```

普通函数调用优先选择非泛型函数；找到泛型函数且省略实参时，从固定参数推断。
支持泛型参数、引用、数组、泛型 Unit 应用及函数指针签名的递归推断。
同一泛型参数的推断必须一致，不自动做数值合并；不从返回目标推断。
信息不足时必须手动提供泛型实参；Unit / Union 构造仍显式给出实参。
推断后仍检查约束；显式与推断调用复用同一具体实例。不增加重载或 Interface 自动推断。
泛型参数名称用作运行期值表达式暂不支持，不能把 T 当值或函数使用。

## 14. Enum

```rust
enum State { Waiting,Running,Stopped };
func main() -> i32 {
    var state:State = State::Running;
    match (state) {
        State::Waiting => { return 0; },
        State::Running => { return 1; },
        _ => { return 2; }
    }
}
```

枚举不携带数据，序号从 0 开始，底层降为 i32，但保持枚举类型身份。
不支持显式指定序号、整数与 Enum 隐式互换或不带前缀的枚举值。
值表达式的一阶名称用于变量，多阶名称用于 Enum::Value 或 module::Enum::Value；
函数调用目标与类型位置使用各自的符号查询，不受此值表达式规则限制。

## 15. Union / Variant

```rust
unit<T> Success { pub value:T; }
unit<E> Failed { pub error:E; }
union<T,E> Result[Success<T>,Failed<E>];

func main() -> i32 {
    var result = Result<i32,i32>{Success<i32>{42}};
    match (value = @mut result) {
        Success<i32> => {
            value->value = 43;
            return value->value;
        },
        Failed<i32> => { return value->error; }
    }
}
```

构造必须恰好给出一个值，精确匹配候选类型，不隐式转换来选择 tag。
候选类型不允许重复；泛型实例化后重新检查重复。
match (name = @object) 创建只读分支 Ref，@mut 创建可变分支 Ref。
必须穷举所有候选，不允许 _、重复类型、非候选类型或按值绑定。
对象必须已有存储；已有 Ref 先解引用再借用，例如 match (y = @mut $reference)。
分支绑定仅在本分支可用，不获得资源所有权；不能移出资源、逃逸引用或在借用期间替换整个 Union。
Union 支持 Interface，但不支持 C ABI。LLVM 布局为 i32 tag + 最大候选大小的对齐内联 payload。
任一候选含资源时，Union 成为 move-only，析构只处理活跃 tag。
更详细的布局规则见 [Union 文档](variant-design.md)。

## 16. 函数指针

```rust
func max(a:i32,b:i32) -> i32 {
    if (a > b) { return a; }
    return b;
}
func<T> identity(value:T) -> T { return value; }
func apply(f:func(i32,i32)->i32) -> i32 { return f(4,7); }
func main() -> i32 {
    var pointer:func(i32,i32)->i32 = std::function(max);
    var specialized = std::function<u32>(identity);
    var result = specialized(cast<u32>(42));
    return apply(pointer);
}
```

std::function(name) 显式取得普通函数地址，不需 import；不允许直接把函数名称当值。
std::function<T1,T2>(name) 按顺序实例化泛型函数，实参必须完整且具体。
类型语法只写参数类型，不写参数名；无返回值为 func()，... 只能在末尾。
签名可递归嵌套，并用于返回值、Unit 字段、数组和 Union 候选。
当前 using 只接受路径目标，using Callback = func(i32)->i32; 尚不可用。
func(T)->T 中的泛型随外层声明实例化，也参与泛型函数调用推断。
类型身份由参数、返回类型、可变参数标志和 ABI 决定，不包含函数名称。
显式 func(...) 类型为内部 ABI；C ABI 指针可从外部函数自动推断，但不能赋给内部 ABI 类型。
支持间接调用与相同签名的复制、赋值；不支持 lambda、null 函数指针、算术或值 match。
不能取得 Interface、cast、alloc 等编译器内建函数的地址。

## 17. 控制流与作用域

if / while 的条件必须为 bool，不采用 C 的整数真值转换。
else if 自动等价于 else { if (...) {...} }。
作用域内变量不向外泄露；独立作用域使用 anonymous {...}，不能裸写 {...} 当语句。

```rust
func main() -> i32 {
    var total = 0;
    for i in [0,5] {
        if (i == 1) { continue; }
        else if (i == 4) { break; }
        else { total = total + i; }
    }
    while (total < 10) { total = total + 1; }
    anonymous {
        val extra = 2;
        total = total + extra;
    }
    return total;
}
```

for name in [start,end] 是整数计数循环，左闭右开，每次加 1，非通用迭代器。
边界需要相符整数类型；不支持步长参数、逆向循环、C 三段式 for。
break / continue 必须位于循环内；match 不会因此成为独立可 break 的循环。
return 只能位于函数或 Interface 内。

### 普通值 match

```rust
func classify(value:i32) -> i32 {
    const one = 1;
    match (value) {
        one => { return 10; },
        2 + 1 => { return 30; },
        _ => { return 0; }
    }
}
func main() -> i32 { return classify(3); }
```

普通 match 的 case 必须为同类型编译期常量，必须有一个 _ 默认分支；
_ 不能重复，分支用逗号分隔。与 Union 的按类型穷举 match 是不同规则。
分支是独立作用域，不发生 C switch 的 fallthrough。
可求值的常量 if / match 仅生成选中分支；恒假 while 和确定零次的 for 可消除。
不能安全展开的常量或未知条件保留运行期控制流，目前不做循环展开。

## 18. Res、move 与自动析构

```rust
import std::mem;

unit Buffer { pub memory:res $mut i32; }

func consume(memory:res $mut i32) -> i32 {
    memory[0] = 42;
    return memory[0];
}
func main() -> i32 {
    var first:res = std::mem::alloc<i32>(4);
    first[0] = 1;
    var borrowed = first;        // 普通只读 Ref，不转移资源
    var observed = borrowed[0];

    var second:res = std::mem::alloc<i32>(1);
    var moved:res = second;       // 所有权转移；second 失效
    var result = consume(moved);  // 传参继续转移；moved 失效

    var third:res = std::mem::alloc<i32>(1);
    var buffer = Buffer{third};   // 资源成员令 Buffer 为 move-only
    var owner:res = buffer;       // 裸 res 也可承接资源聚合类型
    return result + observed;    // 剩余资源按作用域逆序自动清理
}
```

res $T / res $mut T 拥有分配；普通 Ref 不拥有。
局部声明的 :res 从右侧推断完整资源类型，不允许用于函数签名或从普通 Ref 获得所有权。
已有直接 Res 未显式声明 res 时创建普通只读 Ref；临时 Res 必须由明确所有者承接。
函数的 Res 参数、返回值、资源聚合初始化和赋值按预期资源类型转移所有权。
move 后清零源存储并禁止后续使用；分支中任一路径移动，汇合后全体失效。
借用存活时禁止移动所有者；不能返回依赖局部资源的引用。
循环内移动外层资源、部分资源成员 move、全局资源与 alloc 资源元素目前不支持。

退出作用域、return、break、continue 自动逆序清理。
Unit 和固定数组递归析构资源成员；Union 只析构活跃候选。
析构计划在具体类型解析 / 实例化时生成，名称为 <symbol>::deconstruct，可见性固定 pub；
不提供用户手写析构入口。普通 Ref 不析构指向内容，直接 Res 释放后置空。
这套检查不是完整通用的借用检查器，也不等于对任意指针操作提供内存安全保证。

## 19. 内建调用与 C IO

| 调用 | 用途 | 是否需 import |
|---|---|---|
| cast<T>(value) | 显式基本类型 / Ref 转换 | 否 |
| const_c_str("text") | 静态只读 C 字符串，自动追加零 | 否 |
| std::function(name) | 普通函数地址 | 否 |
| std::function<T,...>(name) | 泛型函数实例地址 | 否 |
| std::mem::alloc<T>(count) | 分配 count 个 T，返回 Res | import std::mem; |
| std::mem::dealloc<T>(owner) | 消耗并释放 Res | import std::mem; |

alloc 是低层内存分配，不初始化元素；调用方须先写入再读取。
可能分配失败，不把它当作有容量检查的安全动态数组。
通常依靠自动析构即可；显式 dealloc 后不能再使用所有者，不会重复自动释放。

```rust
import std::mem;
import c_std::io;

func main() -> i32 {
    var memory:res = std::mem::alloc<i32>(1);
    memory[0] = 42;
    c_std::io::printf(const_c_str("value=%d\n"),memory[0]);
    std::mem::dealloc<i32>(memory);
    c_std::io::puts(const_c_str("done"));
    c_std::io::putchar(10);
    return 0;
}
```

当前 c_std::io 提供 printf、scanf、puts、putchar、getchar；
格式字符串、可变参数正确性及目标 CRT 的可用性由调用者负责，不能代替类型安全的格式化库。
GNU Windows 后端可调用对应 scanf；MSVC CRT 的符号可用性需另行确认。

```rust
import c_std::io;
func main() -> i32 {
    var number = 0;
    var scanned = c_std::io::scanf(const_c_str("%d"),@mut number);
    var character = c_std::io::getchar();
    c_std::io::printf(const_c_str("read=%d number=%d char=%d\n"),scanned,number,character);
    return 0;
}
```

scanf 返回成功赋值项数；getchar 的整数返回值保留 EOF 表达空间。
实际输入程序应处理失败与 EOF，这里只展示调用语法。

## 20. Import、using 与模块路径

import 使用模块路径，不是文件字符串；例如 import std::mem;、import c_std::io;。
import module; 后使用 module::symbol，不把所有名称自动展开进本地作用域。
当前目录优先于配置的库目录；目标 triplet 目录优先，无匹配时可回退 universal，
后者主要服务于无源码的声明模块。库搜索设置不是语言语法，详见 CLI 文档。

跨文件示例：

```text
// math.m2
export func twice(value:i32) -> i32 { return value * 2; }

// main.m2
import math;
func main() -> i32 { return math::twice(21); }
```

export import module; 转导出依赖的可搜索符号，例如第三个文件导入桥接模块后，
仍以原模块的限定名称访问符号。源码循环依赖由构建端拒绝。
不允许通过 import 取得其他模块的全局变量。

using 建立符号 / 类型别名，解析后不保留独立运行期 AST 节点。
别名查询优先级低于实际符号；不能借别名绕过可见性或把 Enum 值变成无前缀名称。

```rust
using Number = i32;
func identity(value:Number) -> Number { return value; }
func main() -> i32 {
    var callback:func(i32)->i32 = std::function(identity);
    return callback(42);
}
```

## 21. Attribute 与 C ABI

Attribute 必须写成 #[name,...]#，位于声明和可见性之前；支持多个 attribute 组。
当前公开 attribute 为 c_abi，未知名称产生 warning，重复名称按集合收束。

```rust
#[c_abi]#
export unit CPoint { pub x:i32; pub y:i32; }

#[c_abi]#
export func add_c(a:i32,b:i32) -> i32 { return a + b; }

func main() -> i32 { return add_c(1,2); }
```

c_abi 选择 C ABI，不自动导出；export 决定外部符号可见性。
C ABI 函数使用原名，不使用内部模块修饰名。
C ABI Unit 不允许泛型或 Interface；Union 不允许 C ABI。
C ABI 函数不允许泛型，参数中不能出现非 C ABI Unit（包含经 Ref / 数组包装的情况）。
平台 ABI 布局与寄存器映射由 lowering 决定，不把内部 ABI 规则当作所有平台的 C ABI 标准。

外部 C API 通过声明元数据显式导入，不解析 .h、C 预处理或 C 源码。
人工编写声明 TOML 后用 mlc symbols declarations.toml 转为 .sym；
只声明模块使用 Object = "None"，具体格式见构建管线文档。
符号元数据、对象和泛型体缓存不是语言中的反射或序列化语法。

## 22. 尚未提供的语法与边界

不提供 lambda / 捕获闭包、继承、any、async / await、异常语法、用户手写析构、
宏预处理、函数重载、可变长度数组对象或独立字符串类。
不提供一般意义的编译期用户函数执行；当前常量展开主要覆盖受支持的常量运算。
RAII 不是 GC，Ref 不是完整安全指针；指针索引、C 可变参数和分配失败仍需调用者处理。
标准库目前只有少量声明与内建入口，不承诺完整 C 标准库或平台 SDK 覆盖。

故意错误的写法：

```text
var uninitialized;                  // 必须初始化
var overflow:i8 = 300;               // 数值越界
val fixed = 1; fixed = 2;            // 不可重新赋值
var array:[i32:3] = {1,2,3};         // 数组必须使用 []
if (1) {}                           // 条件必须 bool
export global var shared = 0;       // 全局变量不允许导出
pub func ordinary() {}              // pub 只用于 Interface 或 Unit 字段
self.value                          // self 访问必须 ->
value |> value.method()             // 不允许成员管道
var pointer = ordinary;             // 使用 std::function(ordinary)
match (payload = object) {}         // Union 必须通过 @ / @mut 借用
```
