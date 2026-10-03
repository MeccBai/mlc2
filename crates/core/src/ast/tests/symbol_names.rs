use super::*;

#[test]
fn using_tables_support_forward_names_chains_values_calls_and_types() {
    let ast = valid(
        "using Alias = Point; using Call = make; using Next = Call; using Int = i32; using Ready = State::Ready; unit Point { x:Int; }; enum State { Ready, }; func make() -> i32 { return 1; } func main() { var p = Alias{1}; var x = Next(); var s:State = State::Ready; }",
    );
    assert_eq!(
        ast.symbols.usings[&ast.config.symbol_name("Alias")],
        ast.config.symbol_name("Point")
    );
    assert_eq!(
        ast.symbols.usings[&ast.config.symbol_name("Ready")],
        "State::Ready"
    );
}

#[test]
fn duplicate_names_across_symbol_categories_are_rejected() {
    for source in [
        "unit A {}; func A() {}",
        "enum A { X, }; unit A {};",
        "global var a = 1; func a() {}",
        "using a = b; using a = c;",
        "using a = b; func a() {}",
        "func<T> a(x:T) {} func a() {}",
        "unit A { x:i32; x:i32; };",
        "enum A { X, X, };",
        "unit A { x:i32; }; A::func x() {}",
    ] {
        let ast = invalid_semantics(source);
        assert!(format!("{:?}", ast.config.error_handle().errors).contains("DuplicateSymbol"));
    }
}

#[test]
fn using_cycles_and_non_path_targets_are_not_silently_ignored() {
    for source in [
        "using a = a;",
        "using a = b; using b = a;",
        "using a = $i32;",
    ] {
        invalid_semantics(source);
    }
}

#[test]
fn local_variable_precedes_using() {
    valid("using a = make; func make() {} func main() { var a = 1; var b = a; }");
}
