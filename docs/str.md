# 内建字符串 str

`str` 是内建的 UTF-8 字符串类型，字符串字面量会推断成 `str`，可以直接作为参数和返回值使用。

~~~text
func greeting() -> str { return "hello"; }

func main() {
    var first: str = greeting();
    var current: mut str = first;
    current = "world";
    const bytes: u64 = str::len("中\0文");
    const label: str = "ready";
}
~~~

普通 `str` 绑定默认不可重新赋值，允许用运行时值初始化。`mut str` 允许重新赋值，字符串的字节仍是只读的。需要编译期值时，使用 `const name: str`，也可以写 `var name: const str`。

`text.first_byte()` 返回首个 UTF-8 字节，类型为 `u8`。例如 `"+".first_byte()` 为 `43`，`"中".first_byte()` 为 `228`。空字符串返回 `0`。

`str::len(text)` 返回 UTF-8 字节数，嵌入的 NUL 也计入长度。`str::as_ptr(text)` 返回只读字节指针。`str::from_parts(ptr, len)` 从只读字节指针和长度创建视图，调用者须保证该范围是有效的 UTF-8，且在视图使用期间持续有效。

`const_c_str(text)` 用于把以 NUL 结尾的字符串传给 C。字面量和 `String.view()` 提供结尾 NUL。从原始字节创建视图时，需要由调用者保证结尾 NUL 存在，或者向 C API 显式传入长度。

`String.view()` 返回 `str`，`String::from(text)` 和 `String.append(text)` 接收 `str`。格式化宏的参数声明为 `pattern: str`，格式模板应在编译期确定。

显式声明的字节数组，例如 `var bytes: [i8: 5] = "hello";`，仍然按数组复制和修改。
