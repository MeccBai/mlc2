use super::*;

valid_case!(
    unconstrained_generic_call,
    "func<T> identity(value:T) -> T { return value; } func main() { var a = identity<i32>(1); }"
);
valid_case!(
    independent_generic_parameters,
    "func<T,U> second(a:T,b:U) -> U { return b; } func main() { var a = second<i32,bool>(1,true); }"
);
valid_case!(
    integer_constraint_accepts_integer,
    "generic Number { std::generic::is_integer; }; func<T:Number> identity(a:T) -> T { return a; } func main() { var a = identity<i32>(1); }"
);
valid_case!(
    combined_numeric_constraints,
    "generic Small { std::generic::is_integer; std::generic::max_bits<16>; }; func<T:Small> identity(a:T) -> T { return a; } func main() { var x:i8 = 1; var result = identity<i8>(x); }"
);
valid_case!(
    generic_reference_function,
    "func<T> identity(a:$T) -> $T { return a; } func main() { var x = 1; var p = identity<i32>(@x); }"
);
valid_case!(
    generic_mutable_reference_function,
    "func<T> identity(a:$mut T) -> $mut T { return a; } func main() { var x = 1; var p = identity<i32>(@mut x); }"
);
valid_case!(
    generic_unit_initializer,
    "unit Box<T> { pub value:T; }; func main() { var boxed = Box<i32>{1}; var value = boxed.value; }"
);
valid_case!(
    generic_unit_reference_member,
    "unit Box<T> { pub value:$T; }; func main() { var x = 1; var boxed = Box<i32>{@x}; var value = $boxed.value; }"
);
valid_case!(
    generic_signature_and_body_have_distinct_owners,
    "func<T> first(a:T) -> T { var local:T = a; return local; } func<T> second(a:T) -> T { return first<T>(a); } func main() { var a = second<i32>(1); var b = second<bool>(true); }"
);
valid_case!(
    recursive_generic_function,
    "func<T> recursive(a:T) -> T { return recursive<T>(a); } func main() { var a = recursive<i32>(1); }"
);
valid_case!(
    mutually_recursive_generic_functions,
    "func<T> first(a:T) -> T { return second<T>(a); } func<T> second(a:T) -> T { return first<T>(a); } func main() { var a = first<i32>(1); }"
);
valid_case!(
    generic_control_flow,
    "func<T> run(value:T) { if (value == value) { for i in [0,2] { match (value) { value => { var copy:T = value; }, _ => {} } } } } func main() { run<i32>(1); }"
);

semantic_case!(unknown_generic_constraint, "func<T:Missing> wrong(a:T) {}");
semantic_case!(duplicate_generic_parameter, "func<T,T> wrong() {}");
semantic_case!(
    parameter_cannot_shadow_generic_name,
    "func<T> wrong(T:i32) {}"
);
semantic_case!(
    too_few_generic_arguments,
    "func<T,U> value(a:T) -> T { return a; } func main() { value<i32>(1); }"
);
semantic_case!(
    too_many_generic_arguments,
    "func<T> value(a:T) -> T { return a; } func main() { value<i32,bool>(1); }"
);
semantic_case!(
    generic_argument_type_must_exist,
    "func<T> value(a:T) -> T { return a; } func main() { value<Missing>(1); }"
);
semantic_case!(
    generic_constraint_rejects_boolean,
    "generic Number { std::generic::is_integer; }; func<T:Number> value(a:T) -> T { return a; } func main() { value<bool>(true); }"
);
semantic_case!(
    generic_unit_argument_count,
    "unit Box<T> { value:T; }; func main() { var boxed = Box<i32,bool>{1}; }"
);
semantic_case!(
    generic_type_is_not_a_value,
    "func<T> wrong() { var a = T; } func main() { wrong<i32>(); }"
);

#[test]
fn specialization_cache_reuses_instances_but_separates_types() {
    let ast = valid(
        "func<T> value(a:T) -> T { return a; } func main() { var a = value<i32>(1); var b = value<i32>(2); var c = value<bool>(true); }",
    );
    assert_eq!(ast.symbols.function_instances.len(), 2);
    assert_eq!(ast.body.len(), 3);
    for body in ast.symbols.function_instances.values() {
        let symbol = ast.symbols.functions.get(body.symbol);
        assert!(symbol.generics.is_empty());
        assert!(symbol.generic_map.is_empty());
        assert!(!symbol.params[0].0.is_generic(&ast.symbols.types));
        assert!(!symbol.ret_type.unwrap().is_generic(&ast.symbols.types));
    }
}
