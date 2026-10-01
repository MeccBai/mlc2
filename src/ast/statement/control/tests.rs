use std::rc::Rc;

use crate::ast::{
    AbstractSyntaxTree, Function,
    config::Config,
    expression::{CompAtom, Expression, operators::Operator},
    statement::{MatchPattern, Statement},
};
use crate::error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError, ResolveError};

fn compile(source: &str) -> AbstractSyntaxTree {
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    AbstractSyntaxTree::new(
        Config::new(
            Vec::new(),
            String::new(),
            String::new(),
            ErrorHandle::new("test".into()),
        ),
        module.unwrap(),
    )
}

fn body(ast: &AbstractSyntaxTree) -> &[Statement] {
    let Function::Func(function) = &ast.body[0] else {
        panic!("function expected");
    };
    &function.body
}

fn assert_ok(ast: &AbstractSyntaxTree) {
    assert!(
        ast.config.error_handle().errors.is_empty(),
        "{:?}",
        ast.config.error_handle().errors
    );
}

#[test]
fn if_comparison_returns_boolean_and_branches_are_independent() {
    let ast = compile(
        "func main() { if (1 < 2) { var local = 1; } else { var local = 2; } var local = 3; }",
    );
    assert_ok(&ast);
    let Statement::IfBlock(block) = &body(&ast)[0] else {
        panic!("if expected");
    };
    assert_eq!(block.then_branch.len(), 1);
    assert_eq!(block.else_branch.as_ref().unwrap().len(), 1);
    assert!(matches!(body(&ast)[1], Statement::VariableDecl(_)));
}

#[test]
fn invalid_if_condition_reports_condition_span_without_parsing_branches() {
    let source = "func main() { if (123) { var a = unknown; } }";
    let ast = compile(source);
    let start = source.find("123").unwrap();
    assert!(ast.config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::ExpressionMustConditional),
        (start..start + 3).into()
    )));
    assert_eq!(ast.config.error_handle().errors.len(), 1);
    assert!(matches!(body(&ast)[0], Statement::Poison));
}

#[test]
fn numeric_for_lowers_to_shared_counter_less_condition_and_increment() {
    let ast = compile("func main() { for i in [0, 3] { var value = i; } var i = 10; }");
    assert_ok(&ast);
    let Statement::ForBlock(block) = &body(&ast)[0] else {
        panic!("for expected");
    };
    let Statement::VariableDecl(counter) = block.init.as_deref().unwrap() else {
        panic!("counter expected");
    };
    assert_eq!(counter.var_type.format(&ast.symbols.types), "i32");
    let Expression::CompositeE(condition) = &*block.condition else {
        panic!("comparison expected");
    };
    assert_eq!(condition.operators, [Operator::Less]);
    let CompAtom::VarValueA(value) = &condition.members[0] else {
        panic!("counter expected");
    };
    assert!(Rc::ptr_eq(counter, value));
    let Statement::Assignment(step) = block.execution.as_deref().unwrap() else {
        panic!("increment expected");
    };
    let Expression::VarValueE(step_counter) = &*step.variable else {
        panic!("counter expected");
    };
    assert!(Rc::ptr_eq(counter, step_counter));
    let Statement::VariableDecl(local) = &block.stmts[0] else {
        panic!("local expected");
    };
    let Expression::VarValueE(value) = &*local.init_val else {
        panic!("counter use expected");
    };
    assert!(Rc::ptr_eq(counter, value));
}

#[test]
fn for_rejects_noninteger_bounds_and_shadowing() {
    for (source, error) in [
        (
            "func main() { for i in [true, 3] {} }",
            IllegalUseError::ForBoundMustBeInteger,
        ),
        (
            "func main() { for i in [0, 1.5] {} }",
            IllegalUseError::ForBoundMustBeInteger,
        ),
        (
            "func main() { var i = 1; for i in [0, 3] {} }",
            IllegalUseError::DuplicateVariable { name: "i".into() },
        ),
    ] {
        let ast = compile(source);
        assert!(ast.config.is_poisoned());
        // Confirm the exact category; the original source span is checked separately.
        let span = match error {
            IllegalUseError::DuplicateVariable { .. } => {
                let start = source.find("for i").unwrap() + 4;
                (start..start + 1).into()
            }
            _ => {
                let value = if source.contains("true") {
                    "true"
                } else {
                    "1.5"
                };
                let start = source.find(value).unwrap();
                (start..start + value.len()).into()
            }
        };
        assert!(
            ast.config
                .error_handle()
                .errors
                .contains(&ErrorInfo::new(CompileError::IllegalUse(error), span))
        );
        assert_eq!(ast.config.error_handle().errors.len(), 1);
    }
}

#[test]
fn match_retains_default_and_isolates_branch_locals() {
    let ast = compile(
        "func main() { var x = 1; match (x) { 1 => { var local = 1; }, 2 => { var local = 2; }, _ => { var local = 3; } } var local = 4; }",
    );
    assert_ok(&ast);
    let Statement::MatchBlock(block) = &body(&ast)[1] else {
        panic!("match expected");
    };
    assert_eq!(block.branches.len(), 3);
    assert!(matches!(block.branches[0].0, MatchPattern::Value(_)));
    assert!(matches!(block.branches[2].0, MatchPattern::Default));
}

#[test]
fn match_type_mismatch_reports_branch_span_and_stops_body_parsing() {
    let source = "func main() { match (1) { true => { var a = unknown; } } }";
    let ast = compile(source);
    let start = source.find("true").unwrap();
    assert!(ast.config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
            expected: "i32".into(),
            found: "bool".into()
        }),
        (start..start + 4).into()
    )));
    assert_eq!(ast.config.error_handle().errors.len(), 1);
}

#[test]
fn branch_values_ignore_value_qualifiers_but_not_integer_width() {
    assert_ok(&compile(
        "func main() { val x = 1; match (x) { 1 => {}, _ => {} } }",
    ));
    let ast = compile("func main() { var x:u8 = 1; match (x) { 1 => {} } }");
    assert!(ast.config.is_poisoned());
}

#[test]
fn duplicate_default_and_out_of_scope_counter_are_rejected() {
    let ast = compile("func main() { match (1) { _ => {}, _ => {} } }");
    assert!(ast.config.is_poisoned());
    let source = "func main() { for i in [0, 1] {} var value = i; }";
    let ast = compile(source);
    let start = source.rfind('i').unwrap();
    assert!(ast.config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(ResolveError::UnknownVariable),
        (start..start + 1).into()
    )));
}

#[test]
fn instantiated_function_body_traverses_if_for_and_match() {
    let ast = compile(
        "func<T> run(value:T) { if (value == value) { for i in [0, 2] { match (value) { value => { var local:T = value; }, _ => {} } } } } func main() { run<i32>(1); }",
    );
    assert_ok(&ast);
    assert_eq!(ast.symbols.function_instances.len(), 1);
    let instance = ast.symbols.function_instances.values().next().unwrap();
    assert!(matches!(instance.body[0], Statement::IfBlock(_)));
}
