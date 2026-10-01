//! Desired behavior that is not implemented yet. Keep executable reproductions,
//! rather than treating incorrect acceptance or a panic as a successful test.
//! Run explicitly with: cargo test ast::tests::known_gaps -- --ignored
use super::*;

#[test]
fn mutable_variable_assignment_does_not_panic() {
    valid("func main() { var a = 1; a = 2; }");
}

#[test]
fn enum_variant_value_builds_ast() {
    valid("enum State { Ready, Busy, }; func main() { var state:State = State::Ready; }");
}

#[test]
fn owner_qualified_interface_call_resolves() {
    valid(
        "unit Point {}; Point::func create() -> i32 { return 1; } func main() { var result = Point::create(); }",
    );
}

#[test]
fn owner_qualified_generic_interface_call_resolves() {
    valid(
        "unit Point {}; Point::func<T> choose(value:$T) -> $T { return value; } func main() { var a = 1; var p = Point::choose<i32>(@a); }",
    );
}

#[test]
fn same_named_interfaces_keep_distinct_owners_and_generic_bindings() {
    let ast = valid(
        "unit A {}; unit B {}; A::func<T> choose(value:T) -> T { return value; } B::func<T> choose(value:T) -> T { return value; } A::func create() -> i32 { return 1; } B::func create() -> bool { return true; } func main() { var a = A::choose<i32>(1); var b = B::choose<bool>(true); var c = A::create(); var d = B::create(); }",
    );
    let a = ast
        .symbols
        .generics
        .interfaces
        .get_by_name(&ast.config.symbol_name("A::choose"))
        .unwrap();
    let b = ast
        .symbols
        .generics
        .interfaces
        .get_by_name(&ast.config.symbol_name("B::choose"))
        .unwrap();
    assert_ne!(a, b);
    assert_ne!(
        ast.symbols.generics.interfaces.get(a).generic_map["T"],
        ast.symbols.generics.interfaces.get(b).generic_map["T"],
    );
    assert_eq!(ast.symbols.interface_instances.len(), 2);
    for (index, body) in &ast.symbols.interface_instances {
        assert_eq!(body.symbol, *index);
        let symbol = ast.symbols.interfaces.get(*index);
        assert_eq!(symbol.ret_type, Some(symbol.params[0].0));
        assert_eq!(
            ast.symbols.interfaces.get_by_name(&symbol.name),
            Some(*index)
        );
    }
}

#[test]
fn function_argument_count_is_checked() {
    invalid_semantics("func value(a:i32) -> i32 { return a; } func main() { value(); }");
}

#[test]
fn function_argument_types_are_checked() {
    invalid_semantics("func value(a:i32) -> i32 { return a; } func main() { value(true); }");
}

#[test]
fn return_type_is_checked() {
    invalid_semantics("func value() -> i32 { return true; }");
}

#[test]
fn break_outside_loop_is_rejected() {
    invalid_semantics("func main() { break; }");
}

#[test]
fn nested_generic_unit_argument_is_specialized_in_function_signature() {
    let ast = valid(
        "unit Box<T> { value:T; }; func<T> value(a:Box<T>) -> Box<T> { return a; } func main() { var result = value<i32>(Box<i32>{1}); }",
    );
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let symbol = ast.symbols.functions.get(instance.symbol);
    let expected = ast
        .symbols
        .types
        .get_by_name(&ast.config.symbol_name("Box<i32>"))
        .unwrap();
    assert_eq!(symbol.params[0].0, expected);
    assert_eq!(symbol.ret_type, Some(expected));
}
