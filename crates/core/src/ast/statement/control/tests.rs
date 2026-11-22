use std::rc::Rc;

#[test]
fn match_requires_default_even_for_complete_boolean_cases() {
    for source in [
        "func main() { match (true) { true => {}, false => {} } }",
        "func main() { match (1) {} }",
        "func main() { match (1) { 1 => {} } }",
    ] {
        let ast = compile(source);
        let start = source.find("match (").unwrap() + 7;
        let end = source[start..].find(')').unwrap() + start;
        assert_eq!(ast.config.error_handle().errors.len(), 1);
        assert!(ast.config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::IllegalUse(IllegalUseError::MissingDefaultBranch),
            (start..end).into(),
        )));
    }
}

use crate::diagnostic::error::{
    CompileError, ErrorHandle, ErrorInfo, IllegalUseError, ResolveError,
};
use crate::{
    ast::{
        AnalyzedAst, Function,
        config::Config,
        expression::Expression,
        statement::{MatchPattern, Statement},
    },
    diagnostic::warning::WarningHandle,
};

fn compile(source: &str) -> AnalyzedAst {
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    AnalyzedAst::new(
        Config::new(
            crate::ast::config::FileId::new(0),
            Vec::new(),
            String::new(),
            String::new(),
            ErrorHandle::new("test".into()),
            WarningHandle::new("test".into()),
        ),
        AnalyzedAst::with_iterator_fixture(module.unwrap(), source),
    )
}

fn body(ast: &AnalyzedAst) -> &[Statement] {
    let Function::Func(function) = &ast.body[0] else {
        panic!("function expected");
    };
    &function.body
}

fn assert_ok(ast: &AnalyzedAst) {
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
fn for_captures_the_iterator_and_binds_a_fresh_element() {
    let ast = compile("func main() { for i in test_range(0, 3) { var value = i; } var i = 10; }");
    assert_ok(&ast);
    let Statement::ForBlock(block) = &body(&ast)[0] else {
        panic!("for expected");
    };
    let Statement::VariableDecl(iterator) = block.init.as_deref().unwrap() else {
        panic!("iterator expected");
    };
    assert_eq!(iterator.var_type.format(&ast.symbols.types), "::TestRange");
    assert!(matches!(&*block.condition, Expression::FuncCallE(..)));
    assert!(matches!(
        block.execution.as_deref(),
        Some(Statement::Expression(Expression::FuncCallE(..)))
    ));
    let Statement::VariableDecl(element) = &block.stmts[0] else {
        panic!("element expected");
    };
    let Statement::VariableDecl(local) = &block.stmts[1] else {
        panic!("local expected");
    };
    let Expression::VarValueE(value, ..) = &*local.init_val else {
        panic!("element use expected");
    };
    assert!(Rc::ptr_eq(element, value));
}

#[test]
fn for_rejects_noniterators_and_shadowing() {
    let ast = compile("func main() { for i in true {} }");
    assert!(matches!(
        ast.config.error_handle().first_error().unwrap(),
        CompileError::IllegalUse(IllegalUseError::InvalidIterator { .. })
    ));
    let ast = compile("func main() { var i = 1; for i in test_range(0, 3) {} }");
    assert!(matches!(
        ast.config.error_handle().first_error().unwrap(),
        CompileError::IllegalUse(IllegalUseError::DuplicateVariable { .. })
    ));
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
    let ast = compile("func main() { var x:u8 = 1; match (x) { 1 => {}, _ => {} } }");
    assert!(ast.config.is_poisoned());
}

#[test]
fn duplicate_default_and_out_of_scope_counter_are_rejected() {
    let ast = compile("func main() { match (1) { _ => {}, _ => {} } }");
    assert!(ast.config.is_poisoned());
    let source = "func main() { for i in test_range(0, 1) {} var value = i; }";
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
        "func<T> run(value:T) { if (value == value) { for i in test_range(0, 2) { match (value) { cast<T>(1) => { var local:T = value; }, _ => {} } } } } func main() { run<i32>(1); }",
    );
    assert_ok(&ast);
    assert_eq!(ast.symbols.function_instances.len(), 1);
    let instance = ast.symbols.function_instances.values().next().unwrap();
    assert!(matches!(instance.body[0], Statement::IfBlock(_)));
}
