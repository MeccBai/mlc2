# MLC2

## 定义变量

```
var a = 10; //自动推断
var b:i32 = 10; //手动指定
const c:i32 = 10;//常量定义

var value:i32 = 1;
var reference:$i32 = @value; // @ 默认创建不可变引用
var mutable_reference:$mut i32 = @mut value; // 可变引用必须显式标记
$mutable_reference = 10;    // $expr 解引用，此时 value == 10

global var counter:i32 = 0;
global const limit:i32 = 10;
```

全局变量不能使用 `export` 或 `api`；它们只在当前编译模块内部参与解析和生成。

## 定义函数
```
func test(a:i8) -> i8 {
    return a + 10;
}
```

## 定义单元
```
unit Point {
    pub x: i32; // 可在单元外访问
    y: i32;
}

Point::func add (self, param1:i8) ->i8 {
    return self->x + param1;
}

Point::func set_x(mut self, value:i32) {
    self->x = value;
}

```

`self` 默认不可变；需要修改实例时必须显式写成 `mut self`。方法定义使用
`Type::func method(...)`，成员方法不能作为管道目标。

## 控制流
```
var a = 8;
match (a) {
    1 => {...},
    2 => {...},
    _ => {...}
}

if (a) {

}
else {

}

for i in [0,10] { // 左闭右开：[0, 10)
    break;
}

while (a > 10) {
    continue;
}

return 0;

```

## 初始化与枚举

```
var numbers = [1, 2, 3]; // 数组
var point = Point{1, 3};  // 单元初始化列表

enum State {
    Waiting,
    Running,
};

var state:State = State::Waiting;
```

枚举不携带数据，底层类型固定为 `i32`。

## 函数调用
```
func test1(a:u8,b:u8,c:u8) ->i8 {
    var d = a + b + c;
    return d;
}

//type 1:
var c = test1(5,3,2);

//type 2:
var c = {5,3,2} |> test1;


//compo type:
func test1(a:u8,b:u8,c:u8) ->i8 {
    var d = a + b + c;
    return d;
}
func test2(a:u8,b:u8) -> i8 {
    var d = a + b;
    return d;
}

var sum =  {
    5, 
    {1,2,3} |> test1
} |> test2;


unit One {

};

One::func of() {}

var t:One = One{};
t |> of;       //允许：调用自由函数 of(t)
t |> t.of();   //不允许：成员函数不能作为管道目标


```

## 泛型
```
generic type1 {
    std::generic::is_integer; //是整形
    std::generic::max_bits<16>;//最大16位
};

func<T:type1> test(a:T) ->i8 {
    return a + 1;
}


generic type2 {
    pub func add(self, param1:i8) -> i8;//要求type2必须有一个对应接口
};

generic type3 {
    std::generic::max_bits<16>;//最大16位
    pub func add(self, param1:i8) -> i8;
};//混合要求


```

泛型参数必须在声明处显式列出；约束写作 `<T:type1>`，无约束参数写作 `<T>`。

## Attribute 与 C ABI

```
[[c_abi]]
export func device_entry(value:i32) -> i32 {
    return value;
}
```

`[[c_abi]]` 指定 C ABI；是否对链接器可见仍由 `export`（或同时具有
`pub + export` 语义的 `api`）决定。

## 语义控制
```
//pub 可在单元外部调用
//export 可在文件外被调用
//api pub+export语义
//默认不可被外部调用


unit Point {
    x: i32;
    y: i32;
}

Point::func add (self, param1:i8) ->i8 {
    return self->x + param1;
}//只能在其他接口里调用

Point::func test(self) {
    self.add(1);
}

```

```
pub Point::func add2 (self, param1:i8) ->i8 {
    return self->x + param1;
}//可在类外调用


var t = Point a;
a.add2(10);

```
a.mc :
```
export Point::func add3 (self, param1:i8) ->i8 {
    return self->x + param1;
} //可在文件外被调用，但依然不能被外部调用

var t = Point a;
a.add3(10);//错误，不允许

```
b.mc :
```
Point::func add4 (self, param1:i8) ->i8 {
    return self.add3(10); //允许
} 

```


允许的export:
```
export unit a {

};
export a::func at() {

}
export func atx() {
    
}
```
允许接力export。

a.mc:
```
export func at() {}
```
b.mc: 
```
export import a;
```
c.mc: 
```
import b;//允许使用a的at函数
func axx() {
    a::at();
}
```


