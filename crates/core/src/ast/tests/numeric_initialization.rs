use super::*;

#[test]
fn bit_builtins_fold_bits_without_numeric_conversion() {
    use crate::ast::{expression::Expression, statement::Statement};
    let ast = valid(
        "func main(){const a=std::bit::and(-1,15);const b=std::bit::xor(7,3);const c=std::bit::or(4,3);const d=std::bit::not(cast<u8>(0));const e=std::bit::shl(1,3);const f=std::bit::shr(-1,1);const g=std::bit::or(cast<f32>(1.0),cast<u32>(0));const h=std::bit::and(true,false);const i=std::bit::not(true);}",
    );
    for (statement, expected) in main_body(&ast).iter().zip([
        "15",
        "4",
        "7",
        "255",
        "8",
        "2147483647",
        "1065353216",
        "false",
        "false",
    ]) {
        let Statement::VariableDecl(variable) = statement else {
            panic!("expected variable")
        };
        let Expression::ConstValueE(value, _) = &*variable.init_val else {
            panic!("expected bit literal: {:?}", variable.init_val)
        };
        assert_eq!(value.value, expected);
    }
}

#[test]
fn bit_builtins_require_equal_width_base_operands_and_exact_arity() {
    for source in [
        "func main(){var n=std::bit::and(cast<u8>(1),cast<u16>(1));}",
        "func main(){var n=std::bit::or(1.0,1);}",
        "func main(){var n=std::bit::xor(\"a\",\"b\");}",
        "func main(){var x=1;var n=std::bit::and(@x,@x);}",
        "func main(){var n=std::bit::shl(1);}",
        "func main(){var n=std::bit::not(1,2);}",
        "func main(){var n=std::bit::and<i32>(1,2);}",
    ] {
        invalid_semantics(source);
    }
    valid(
        "func main(){var a:i32=1;var b:u32=2;var n:u32=std::bit::or(a,b);var f:f32=cast<f32>(1.0);var bits:u32=std::bit::xor(f,b);}",
    );
}

#[test]
fn bit_builtins_keep_nested_generic_calls_unambiguous() {
    valid(
        "unit Box<T>{pub value:T;};func<T> identity(value:T)->T{return value;}func main(){var value=identity<Box<Box<i32>>>(Box<Box<i32>>{Box<i32>{1}});var n=std::bit::shl(1,2);}",
    );
    invalid_syntax("func main(){var n=1 << 2;}");
    invalid_syntax("func main(){var n=4 >> 1;}");
}

#[test]
fn size_builtin_replaces_type_queries_with_target_layout_literals() {
    use crate::ast::{expression::Expression, statement::Statement};
    let ast = valid(
        "unit P{pub a:u8;pub b:i64;};func main(){const scalar=size<i32>();const array=size<[i32:3]>();const aggregate=size<P>();const pointer=size<$i8>();var runtime=size<u16>();}",
    );
    for (statement, expected) in
        main_body(&ast)
            .iter()
            .zip([4, 12, 16, std::mem::size_of::<usize>(), 2])
    {
        let Statement::VariableDecl(variable) = statement else {
            panic!("expected variable")
        };
        let Expression::ConstValueE(value, _) = &*variable.init_val else {
            panic!("size must lower directly to a literal")
        };
        assert_eq!(value.value, expected.to_string());
    }
}

#[test]
fn size_builtin_supports_generic_instantiation_and_const_function_bodies() {
    valid(
        "const func<T> width()->u64{return size<T>();}func main(){var a=width<i32>();var b=width<$i8>();}",
    );
}

#[test]
fn size_builtin_uses_target_pointer_width_instead_of_host_width() {
    use crate::ast::{expression::Expression, statement::Statement};
    let source = "func main(){const pointer=size<$i8>();}";
    let mut config = valid("func main(){}").config;
    config.ptr_size = 4;
    config.ptr_align = 4;
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(errors.is_empty());
    let ast = AnalyzedAst::new(config, module.unwrap());
    assert!(!ast.config.is_poisoned());
    let Statement::VariableDecl(variable) = &main_body(&ast)[0] else {
        panic!("expected variable")
    };
    let Expression::ConstValueE(value, _) = &*variable.init_val else {
        panic!("expected size literal")
    };
    assert_eq!(value.value, "4");
    assert_eq!(
        ast.symbols
            .types
            .get(value.ty)
            .unqualified()
            .format(&ast.symbols.types),
        "u32"
    );
}

#[test]
fn size_builtin_rejects_missing_types_extra_types_and_value_arguments() {
    for source in [
        "func main(){var n=size();}",
        "func main(){var n=size<i32,i64>();}",
        "func main(){var n=size<i32>(1);}",
        "func main(){var n=size<Missing>();}",
    ] {
        invalid_semantics(source);
    }
}

#[test]
fn numeric_constants_use_destination_range_and_exact_precision() {
    for declaration in [
        "var x:u32 = 10;",
        "var x:u64 = 4294967296;",
        "var x:i64 = 4294967296;",
        "var x:u8 = 255;",
        "var x:i8 = -128;",
        "var x:u32 = 2*3;",
        "var x:f32 = cast<f32>(1.5);",
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

#[test]
fn floating_narrowing_requires_explicit_cast_even_for_exact_constants() {
    for source in [
        "func main(){ var a:f32=10.0; }",
        "func main(){ val a:f32=10.0; }",
        "func main(){ const a:f32=10.0; }",
        "func main(){ var a:f32=-10.0; }",
        "func main(){ var a:f32=5.0+5.0; }",
        "func main(){ const n=10.0; var a:f32=n; }",
        "global var a:f32=10.0; func main(){}",
        "func main(){ var a:[f32:1]=[10.0]; }",
        "unit P { pub a:f32; }; func main(){var p=P{10.0};}",
        "func main(){var a:f32=cast<f32>(0.0); a=10.0;}",
    ] {
        invalid_semantics(source);
    }
    valid("func main(){var a:f32=cast<f32>(10.0); val b:f32=cast<f32>(0.1); var c:f64=a;}");
    let source = "func main(){var a:f32=10.0;}";
    let ast = invalid_semantics(source);
    let diagnostic = ast.config.error_handle().render(source);
    assert!(
        diagnostic.contains("f32") && diagnostic.contains("f64"),
        "{diagnostic}"
    );
}
