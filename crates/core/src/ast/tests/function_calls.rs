use super::*;
use crate::diagnostic::error::IllegalUseError;

valid_case!(
    zero_argument_function,
    "func run() {} func main() { run(); }"
);
valid_case!(
    typed_fixed_arguments,
    "func run(a:i32,b:bool) {} func main() { run(1,true); }"
);
valid_case!(
    variadic_zero_arguments,
    "func run(...); func main() { run(); }"
);
valid_case!(
    variadic_mixed_arguments,
    "func run(...); func main() { run(1,true,1.5); }"
);
valid_case!(
    variadic_fixed_prefix_only,
    "func run(a:i32,...); func main() { run(1); }"
);
valid_case!(
    variadic_fixed_prefix_and_tail,
    "func run(a:i32,...); func main() { run(1,true,1.5); }"
);
valid_case!(
    pipeline_fixed_arguments_are_checked,
    "func add(a:i32,b:i32) -> i32 { return a + b; } func main() { var a = {1,2} |> add; }"
);
valid_case!(
    generic_variadic_fixed_prefix,
    "func<T> run(a:T,...); func main() { run<i32>(1,true); }"
);

semantic_case!(
    too_many_function_arguments,
    "func run(a:i32) {} func main() { run(1,2); }"
);
semantic_case!(
    missing_fixed_variadic_argument,
    "func run(a:i32,...); func main() { run(); }"
);
semantic_case!(
    wrong_fixed_variadic_type,
    "func run(a:i32,...); func main() { run(true,1); }"
);
semantic_case!(
    integer_width_must_match,
    "func run(a:u8) {} func main() { run(1); }"
);
semantic_case!(
    mutable_reference_parameter_rejects_immutable_reference,
    "func run(a:$mut i32) {} func main() { var a = 1; run(@a); }"
);
semantic_case!(
    void_expression_cannot_fill_parameter,
    "func nothing() {} func run(a:i32) {} func main() { run(nothing()); }"
);
semantic_case!(
    pipeline_argument_count_is_checked,
    "func run(a:i32,b:i32) {} func main() { 1 |> run; }"
);
semantic_case!(
    pipeline_argument_type_is_checked,
    "func run(a:i32) {} func main() { true |> run; }"
);
semantic_case!(
    generic_call_argument_count_is_checked,
    "func<T> run(a:T) {} func main() { run<i32>(); }"
);
semantic_case!(
    generic_call_argument_type_is_checked,
    "func<T> run(a:T) {} func main() { run<i32>(true); }"
);

#[test]
fn count_mismatch_submits_call_span() {
    assert_error(
        "func run(a:i32) {} func main() { run(); }",
        "run()",
        CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
    );
}

#[test]
fn type_mismatch_submits_call_span() {
    assert_error(
        "func run(a:i32) {} func main() { run(true); }",
        "run(true)",
        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
            expected: "i32".into(),
            found: "const bool".into(),
        }),
    );
}

#[test]
fn invalid_argument_does_not_cascade_into_parameter_errors() {
    assert_error(
        "func run(a:i32) {} func main() { run(missing); }",
        "missing",
        CompileError::Resolve(crate::diagnostic::error::ResolveError::UnknownVariable),
    );
}
