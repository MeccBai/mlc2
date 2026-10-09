use super::*;
use crate::ast::{
    expression::{CompAtom, Expression},
    statement::{MatchPattern, Statement},
};
use std::rc::Rc;

#[test]
fn match_case_requires_a_compile_time_constant() {
    assert_error(
        "func main() { var value = 1; match (value) { value+0 => {} } }",
        "value+0",
        CompileError::IllegalUse(crate::diagnostic::error::IllegalUseError::NonConstantMatchCase),
    );
    valid(
        "func main() { const case_value = 2*3; var value = 6; match (value) { case_value => {}, _ => {} } }",
    );
}

valid_case!(
    nested_if_and_boolean_conditions,
    "func main() { if (true) { if (1 < 2) { var a = 1; } else { var a = 2; } } else {} }"
);
valid_case!(
    while_comparison_and_continue,
    "func main() { var a = 1; while (a < 10) { var current = a; continue; break; } }"
);
valid_case!(
    for_numeric_ranges_and_nested_loops,
    "func main() { for i in [0,10] { for j in [0,2] { var sum = i + j; continue; } break; } }"
);
valid_case!(
    empty_and_reversed_ranges,
    "func main() { for i in [2,2] {} for i in [5,0] {} }"
);
valid_case!(
    for_variable_bounds,
    "func main() { var start = 0; var end = 3; for i in [start,end] { var a = i; } }"
);
valid_case!(
    match_integer_and_default,
    "func main() { var a = 1; match (a) { 1 => {}, 2 => {}, _ => {} } }"
);
valid_case!(
    match_boolean_patterns,
    "func main() { match (true) { true => {}, false => {}, _ => {} } }"
);
valid_case!(
    match_float_patterns,
    "func main() { match (1.5) { 1.5 => {}, _ => {} } }"
);
valid_case!(
    independent_branch_and_loop_scopes,
    "func main() { if (true) { var a = 1; } else { var a = 2; } match (1) { 1 => { var a = 3; }, _ => { var a = 4; } } for a in [0,2] {} var a = 5; }"
);
valid_case!(
    anonymous_scope_is_explicit,
    "func main() { anonymous { var a = 1; anonymous { var b = a; } } var a = 2; }"
);

semantic_case!(if_requires_boolean, "func main() { if (1) {} }");
semantic_case!(while_requires_boolean, "func main() { while (1.5) {} }");
semantic_case!(
    for_requires_integer_start,
    "func main() { for i in [false,2] {} }"
);
semantic_case!(
    for_requires_integer_end,
    "func main() { for i in [0,2.5] {} }"
);
semantic_case!(
    for_requires_consistent_bound_types,
    "func main() { var end:u8 = 3; for i in [0,end] {} }"
);
semantic_case!(
    for_counter_cannot_shadow_outer_variable,
    "func main() { var i = 1; for i in [0,2] {} }"
);
semantic_case!(
    for_counter_does_not_escape,
    "func main() { for i in [0,2] {} var a = i; }"
);
semantic_case!(
    match_requires_consistent_pattern_type,
    "func main() { match (1) { true => {} } }"
);
semantic_case!(
    match_rejects_two_default_branches,
    "func main() { match (1) { _ => {}, _ => {} } }"
);
semantic_case!(
    branch_variable_does_not_escape,
    "func main() { if (true) { var a = 1; } var b = a; }"
);
semantic_case!(
    inner_scope_cannot_shadow_parameter,
    "func main(a:i32) { anonymous { var a = 1; } }"
);

syntax_case!(
    if_requires_parenthesized_condition,
    "func main() { if true {} }"
);
syntax_case!(for_requires_two_bounds, "func main() { for i in [0] {} }");
syntax_case!(
    match_requires_branch_scope,
    "func main() { match (1) { 1 => 2 } }"
);
syntax_case!(bare_anonymous_block_is_forbidden, "func main() { {} }");

#[test]
fn numeric_loop_counter_identity_survives_ast_build() {
    let ast = valid("func main() { for i in [0,2] { var value = i; } }");
    let Statement::ForBlock(block) = &main_body(&ast)[0] else {
        panic!("for expected");
    };
    let Statement::VariableDecl(counter) = block.init.as_deref().unwrap() else {
        panic!("counter expected");
    };
    let Expression::CompositeE(condition) = &*block.condition else {
        panic!("comparison expected");
    };
    let CompAtom::VarValueA(condition_counter) = &condition.members[0] else {
        panic!("counter use expected");
    };
    assert!(Rc::ptr_eq(counter, condition_counter));
    let Statement::VariableDecl(local) = &block.stmts[0] else {
        panic!("local expected");
    };
    let Expression::VarValueE(body_counter) = &*local.init_val else {
        panic!("counter use expected");
    };
    assert!(Rc::ptr_eq(counter, body_counter));
}

#[test]
fn match_default_is_not_an_empty_expression() {
    let ast = valid("func main() { match (1) { 1 => {}, _ => {} } }");
    let Statement::MatchBlock(block) = &main_body(&ast)[0] else {
        panic!("match expected");
    };
    assert!(matches!(block.branches[0].0, MatchPattern::Value(_)));
    assert!(matches!(block.branches[1].0, MatchPattern::Default));
}
