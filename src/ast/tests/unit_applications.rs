use super::*;
use crate::ast::{
    expression::Expression,
    statement::Statement,
    types::{CompileType, ValueType},
};
use std::rc::Rc;

fn concrete_signature(source: &str, expected: &str) -> AbstractSyntaxTree {
    let ast = valid(source);
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let symbol = ast.symbols.functions.get(instance.symbol);
    let expected = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name(expected))
        .unwrap();
    assert_eq!(symbol.params[0].0, expected);
    assert_eq!(symbol.ret_type, Some(expected));
    ast
}

#[test]
fn empty_unit_keeps_unused_generic_argument_until_specialization() {
    concrete_signature(
        "unit Tag<T> {}; func<T> same(a:Tag<T>) -> Tag<T> { return a; } func main() { var result = same<i32>(Tag<i32>{}); }",
        "Tag<i32>",
    );
}

#[test]
fn nested_application_produces_canonical_concrete_type() {
    concrete_signature(
        "unit Box<T> { value:T; }; func<T> same(a:Box<Box<T> >) -> Box<Box<T> > { return a; } func main() { var result = same<i32>(Box<Box<i32> >{Box<i32>{1}}); }",
        "Box<::Box<i32>>",
    );
}

#[test]
fn reference_argument_produces_canonical_concrete_type() {
    concrete_signature(
        "unit Box<T> { value:T; }; func<T> same(a:Box<$T>) -> Box<$T> { return a; } func main() { var x = 1; var result = same<i32>(Box<$i32>{@x}); }",
        "Box<$i32>",
    );
}

#[test]
fn independent_parameters_are_not_confused() {
    concrete_signature(
        "unit Pair<A,B> { first:A; second:B; }; func<T,U> same(a:Pair<U,T>) -> Pair<U,T> { return a; } func main() { var result = same<i32,bool>(Pair<bool,i32>{true,1}); }",
        "Pair<bool,i32>",
    );
}

#[test]
fn constrained_unit_defers_check_until_arguments_are_concrete() {
    concrete_signature(
        "generic Number { std::generic::is_integer; }; unit Box<T:Number> { value:T; }; func<T> same(a:Box<T>) -> Box<T> { return a; } func main() { var result = same<i32>(Box<i32>{1}); }",
        "Box<i32>",
    );
}

semantic_case!(
    concrete_application_still_checks_constraint,
    "generic Number { std::generic::is_integer; }; unit Box<T:Number> { value:T; }; func<T> make() { var box = Box<T>{true}; } func main() { make<bool>(); }"
);

#[test]
fn nested_generic_unit_members_are_specialized() {
    let ast = concrete_signature(
        "unit Box<T> { value:T; }; unit Outer<T> { box:Box<T>; }; func<T> same(a:Outer<T>) -> Outer<T> { return a; } func main() { var result = same<i32>(Outer<i32>{Box<i32>{1}}); }",
        "Outer<i32>",
    );
    let outer = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name("Outer<i32>"))
        .unwrap();
    let boxed = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name("Box<i32>"))
        .unwrap();
    let CompileType::Unit(unit) = ast.symbols.types.get(outer) else {
        panic!("unit expected");
    };
    assert_eq!(unit.members[0].member_type, boxed);
}

#[test]
fn body_locals_and_uses_share_concrete_unit_instance() {
    let ast = valid(
        "unit Box<T> { value:T; }; func<T> same(a:Box<T>) -> Box<T> { var local:Box<T> = a; val frozen:Box<T> = local; return local; } func main() { var result = same<i32>(Box<i32>{1}); }",
    );
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let concrete = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name("Box<i32>"))
        .unwrap();
    let Statement::VariableDecl(local) = &instance.body[0] else {
        panic!("local expected");
    };
    assert_eq!(local.var_type, concrete);
    let Statement::VariableDecl(frozen) = &instance.body[1] else {
        panic!("val expected");
    };
    assert_eq!(
        frozen.var_type.value_type(&ast.symbols.types),
        ValueType::Final
    );
    let Statement::ReturnBlock(ret) = &instance.body[2] else {
        panic!("return expected");
    };
    let Some(Expression::VarValueE(value)) = &ret.value else {
        panic!("variable expected");
    };
    assert!(Rc::ptr_eq(local, value));
}

#[test]
fn recursive_generic_unit_uses_holder_without_recursing_forever() {
    let ast = valid(
        "unit Node<T> { next:$Node<T>; value:T; }; func<T> inspect(a:$Node<T>) -> $Node<T> { return a; } func main() { var node = Node<i32>{null,1}; var result = inspect<i32>(@node); }",
    );
    let concrete = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name("Node<i32>"))
        .unwrap();
    let CompileType::Unit(unit) = ast.symbols.types.get(concrete) else {
        panic!("unit expected");
    };
    let CompileType::Ref(reference) = ast.symbols.types.get(unit.members[0].member_type) else {
        panic!("reference expected");
    };
    assert_eq!(reference.base, concrete);
}
