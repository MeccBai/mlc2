use super::*;
use crate::ast::{
    expression::{Access, Expression, UnaryExpr, operators::Operator},
    statement::Statement,
};
use crate::diagnostic::error::IllegalUseError;

#[test]
fn assignment_left_sides_retain_variables_members_indices_and_dereferences() {
    let ast = valid(
        "unit Point { pub x:i32; }; func main() { var a = 1; a = 2; var point = Point{1}; point.x = 3; var array = [1,2]; array[0] = 4; var p = @mut a; $p = 5; }",
    );
    let assignments = main_body(&ast)
        .iter()
        .filter_map(|statement| match statement {
            Statement::Assignment(assignment) => Some(assignment),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(assignments.len(), 4);
    assert!(matches!(
        &*assignments[0].variable,
        Expression::VarValueE(_)
    ));
    assert!(matches!(
        &*assignments[1].variable,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Member { .. }))
    ));
    assert!(matches!(
        &*assignments[2].variable,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { .. }))
    ));
    assert!(matches!(
        &*assignments[3].variable,
        Expression::UnaryExprE(UnaryExpr::Operator {
            op: Operator::Dereference,
            ..
        })
    ));
    assert!(
        assignments
            .iter()
            .all(|assignment| matches!(&*assignment.value, Expression::ConstValueE(_)))
    );
}

semantic_case!(
    assigning_to_val_is_illegal,
    "func main() { val a = 1; a = 2; }"
);
semantic_case!(
    assigning_to_immutable_reference_is_illegal,
    "func main() { var a = 1; var p = @a; $p = 2; }"
);
semantic_case!(assigning_to_literal_is_illegal, "func main() { 1 = 2; }");
valid_case!(
    generic_assignment_traverses_both_expressions,
    "func<T> set(value:$mut T, next:T) { $value = next; } func main() { var a = 1; set<i32>(@mut a,2); }"
);

valid_case!(
    loop_controls_find_loop_through_nested_scopes,
    "func main() { for i in [0,2] { if (true) { anonymous { continue; } } match (i) { 0 => { break; }, _ => { continue; } } } while (true) { if (false) { break; } } }"
);
semantic_case!(
    continue_outside_loop_is_illegal,
    "func main() { continue; }"
);
semantic_case!(if_is_not_a_loop, "func main() { if (true) { break; } }");
semantic_case!(
    match_is_not_a_loop,
    "func main() { match (1) { _ => { continue; } } }"
);
semantic_case!(
    exiting_loop_restores_outer_scope,
    "func main() { for i in [0,2] { continue; } break; }"
);

valid_case!(
    void_function_can_return_without_value,
    "func main() { return; }"
);
valid_case!(
    return_resolves_function_through_nested_scopes,
    "func value(a:i32) -> i32 { anonymous { if (true) { return a; } } for i in [0,2] { return i; } return 0; } func main() { var result = value(1); }"
);
semantic_case!(
    nonvoid_return_requires_value,
    "func value() -> i32 { return; }"
);
semantic_case!(void_return_rejects_value, "func main() { return 1; }");
semantic_case!(
    nested_return_type_is_checked,
    "func value() -> i32 { if (true) { return false; } }"
);

#[test]
fn invalid_loop_control_submits_error_at_keyword() {
    assert_error(
        "func main() { anonymous { continue; } }",
        "continue;",
        CompileError::IllegalUse(IllegalUseError::LoopControlOutsideLoop),
    );
}

#[test]
fn missing_return_value_submits_error_at_return_statement() {
    assert_error(
        "func value() -> i32 { return; }",
        "return;",
        CompileError::IllegalUse(IllegalUseError::ReturnValueRequired),
    );
}

#[test]
fn invalid_return_type_submits_error_at_value() {
    assert_error(
        "func value() -> i32 { return true; }",
        "true",
        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
            expected: "i32".into(),
            found: "const bool".into(),
        }),
    );
}
