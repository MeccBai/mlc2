use super::*;

#[test]
fn infers_swap_references_and_reuses_the_explicit_instance() {
    let ast = valid(
        "generic Base { std::generic::is_integer; }; func<T:Base> swap(a:$mut T,b:$mut T) { var temporary = $a; $a = $b; $b = temporary; } func main() { var a = 1; var b = 2; swap(@mut a,@mut b); swap<i32>(@mut a,@mut b); }",
    );
    assert_eq!(ast.symbols.function_instances.len(), 1);
}

#[test]
fn infers_multiple_parameters_arrays_units_and_nested_generic_calls() {
    for source in [
        "func<T,U> second(a:T,b:U) -> U { return b; } func main() { var x = second(1,true); }",
        "func<T> first(a:[T:2]) -> T { return a[0]; } func main() { var x = first([1,2]); }",
        "unit Box<T> { pub value:T; }; func<T> read(a:Box<T>) -> T { return a.value; } func main() { var x = read(Box<i32>{1}); }",
        "func<T> inner(a:T) -> T { return a; } func<T> outer(a:T) -> T { return inner(a); } func main() { var x = outer(1); }",
        "func<T> recursive(a:T) -> T { return recursive(a); } func main() { var x = recursive(1); }",
        "func<T> get(a:$T) -> T { return $a; } func main() { var x = 1; var y = get(@mut x); }",
    ] {
        valid(source);
    }
}

#[test]
fn inference_rejects_conflicts_missing_parameters_and_constraints() {
    for (source, message) in [
        (
            "func<T> same(a:T,b:T) {} func main() { same(1,true); }",
            "Conflicting inferred types",
        ),
        (
            "func<T> missing() {} func main() { missing(); }",
            "Cannot infer generic parameter `T`",
        ),
        (
            "generic Integer { std::generic::is_integer; }; func<T:Integer> f(x:T) {} func main() { f(1.0); }",
            "std::generic::is_integer",
        ),
        (
            "func<T> f(a:T,b:T) {} func main() { f(1); }",
            "Argument count does not match",
        ),
        (
            "func<T> f(a:$mut T) {} func main() { var a = 1; f(@a); }",
            "Type mismatch",
        ),
    ] {
        let ast = invalid_semantics(source);
        let diagnostic = ast.config.error_handle().render(source);
        assert!(diagnostic.contains(message), "{diagnostic}");
    }
    invalid_semantics("func<T> f(x:T) {} func main() { var f = 1; f(2); }");
}
