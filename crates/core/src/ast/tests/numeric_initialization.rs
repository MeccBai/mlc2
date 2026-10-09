use super::*;

#[test]
fn numeric_constants_use_destination_range_and_exact_precision() {
    for declaration in [
        "var x:u32 = 10;",
        "var x:u64 = 4294967296;",
        "var x:i64 = 4294967296;",
        "var x:u8 = 255;",
        "var x:i8 = -128;",
        "var x:u32 = 2*3;",
        "var x:f32 = 1.5;",
        "var x:f64 = 42;",
        "var x:i32 = 42.0;",
        "var x:f32 = 16777216;",
        "var x:f64 = 9007199254740992;",
        "var x:[u64:2] = [4294967296,1];",
        "unit P { value:u64; }; var x:P = {4294967296};",
        "const a = 42; var x:u64 = a;",
    ] {
        // Unit definitions belong outside the function.
        let source = if declaration.starts_with("unit ") {
            let (unit, declaration) = declaration.split_once("; var").unwrap();
            format!("{unit}; func main() {{ var{declaration} }}")
        } else {
            format!("func main() {{ {declaration} }}")
        };
        valid(&source);
    }
}

#[test]
fn numeric_initializers_reject_overflow_and_precision_loss() {
    for declaration in [
        "var x:u32 = -1;",
        "var x:u8 = 256;",
        "var x:i8 = 128;",
        "var x:i8 = -129;",
        "var x:u32 = 4294967296;",
        "var x:f32 = 0.1;",
        "var x:f32 = 16777217;",
        "var x:f64 = 9007199254740993;",
        "var x:i32 = 1.5;",
        "var x:u32 = -1.0;",
        "var x:f32 = 1e100;",
        "var x:[u8:2] = [1,256];",
        "var x:[f32:1] = [0.1];",
    ] {
        invalid_semantics(&format!("func main() {{ {declaration} }}"));
    }
}

#[test]
fn runtime_initializers_require_lossless_domain_conversions() {
    for (from, to, accepted) in [
        ("i8", "i32", true),
        ("u8", "i32", true),
        ("u32", "i64", true),
        ("i32", "u32", false),
        ("u32", "i32", false),
        ("i64", "i32", false),
        ("f32", "f64", true),
        ("f64", "f32", false),
        ("i32", "f64", true),
        ("i32", "f32", false),
        ("u64", "f64", false),
        ("f64", "i64", false),
        ("bool", "i32", false),
    ] {
        let source = format!("func f(input:{from}) {{ var output:{to} = input; }}");
        if accepted {
            valid(&source);
        } else {
            invalid_semantics(&source);
        }
    }
    valid("func main() { var x:f32 = cast<f32>(0.1); var y:u32 = cast<u32>(-1); }");
}
