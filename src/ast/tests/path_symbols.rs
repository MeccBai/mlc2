use super::*;
use crate::ast::symbols::PathSymbol;
use crate::parser::out::TempPath;

fn error_contains(source: &str, error: &str) {
    let ast = invalid_semantics(source);
    assert!(format!("{:?}", ast.config.error_handle().errors).contains(error));
}

#[test]
fn callable_paths_as_values_are_recognized_but_not_supported() {
    error_contains(
        "func item() {} func main() { var x = item; }",
        "UnsupportedSymbolValue",
    );
    error_contains(
        "func<T> item(a:T) {} func main() { var x = item; }",
        "UnsupportedSymbolValue",
    );
    error_contains(
        "unit A {}; A::func item() {} func main() { var x = A::item; }",
        "UnsupportedSymbolValue",
    );
}

#[test]
fn variables_do_not_fall_through_to_same_named_functions() {
    error_contains(
        "func item() {} func main() { var item = 1; item(); }",
        "SymbolNotCallable",
    );
    error_contains(
        "func<T> item(a:T) {} func main() { var item = 1; item<i32>(1); }",
        "SymbolNotCallable",
    );
    error_contains(
        "global var item = 1; func item() {} func main() { item(); }",
        "DuplicateSymbol",
    );
}

#[test]
fn generic_parameter_takes_priority_over_callable() {
    error_contains(
        "func T() {} func<T> item(a:T) { T(); } func main() { item<i32>(1); }",
        "TypeUsedAsValue",
    );
}

#[test]
fn enum_values_preserve_type_and_ordinal_and_precede_globals() {
    let mut ast = valid(
        "enum State { Ready, Busy, }; global var fallback = 7; func main() { const state:State = State::Busy; }",
    );
    let fallback = ast.symbols.globals["fallback"].clone();
    ast.symbols.globals.insert("State::Busy".into(), fallback);
    let path = TempPath {
        segments: vec!["State".into(), "Busy".into()],
    };
    let Some(PathSymbol::EnumValue(value)) = ast.symbols.resolve_path(&ast.config, &path, None)
    else {
        panic!("enum value must precede the global binding");
    };
    assert_eq!(value.value, 1);
    assert_eq!(
        ast.symbols
            .types
            .get(value.enum_type)
            .format(&ast.symbols.types),
        ast.config.symbol_name("State")
    );
    let super::super::statement::Statement::VariableDecl(variable) = &main_body(&ast)[0] else {
        panic!("variable expected");
    };
    let super::super::expression::Expression::ConstValueE(initializer) = &*variable.init_val else {
        panic!("enum initializer must be a typed constant");
    };
    assert_eq!(initializer.value, "1");
    assert_eq!(initializer.ty, value.enum_type);
}

#[test]
fn invalid_enum_variants_and_enum_calls_are_rejected() {
    error_contains(
        "enum State { Ready, }; func main() { var x = State::Missing; }",
        "UnknownVariable",
    );
    error_contains(
        "enum State { Ready, }; func main() { State::Ready(); }",
        "SymbolNotCallable",
    );
}

#[test]
fn interface_has_priority_over_function_and_preserves_generic_kind() {
    let mut ast =
        valid("unit A {}; A::func<T> item(a:T) -> T { return a; } func item() {} func main() {}");
    let function = ast.symbols.functions.get_by_name(&"item".into()).unwrap();
    let mut symbol = ast.symbols.functions.get(function).clone();
    symbol.name = ast.config.symbol_name("A::item");
    ast.symbols.functions.insert(symbol.name.clone(), symbol);
    let result = ast.symbols.resolve_path(
        &ast.config,
        &TempPath {
            segments: vec!["A".into(), "item".into()],
        },
        None,
    );
    assert!(matches!(
        result,
        Some(PathSymbol::Interface { generic: true, .. })
    ));
    let result = ast.symbols.resolve_path(
        &ast.config,
        &TempPath {
            segments: vec!["item".into()],
        },
        None,
    );
    assert!(matches!(
        result,
        Some(PathSymbol::Function { generic: false, .. })
    ));
}
