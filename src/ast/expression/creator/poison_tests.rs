use crate::ast::{
    SymbolTable,
    config::Config,
    expression::{Expression, operators::Operator},
};
use crate::error::{CompileError, ErrorHandle, ErrorInfo, ResolveError};
use crate::parser::out::{TempExpr, TempLiteralKind, TempPath};

#[test]
fn poisoned_child_stops_parent_and_later_expressions_without_cascades() {
    let mut config = Config::new(
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let span = (2..9).into();
    let expression = Expression::new(
        &mut config,
        (
            TempExpr::Unary {
                op: Operator::MutOf,
                value: Box::new((
                    TempExpr::Unary {
                        op: Operator::Dereference,
                        value: Box::new((
                            TempExpr::Path(TempPath {
                                segments: vec!["missing".into()],
                            }),
                            span,
                        )),
                    },
                    span,
                )),
            },
            span,
        ),
        &mut symbols,
        None,
    );
    assert!(expression.is_poisoned());
    assert!(!expression.is_null());
    assert!(
        expression
            .type_inference(&mut config, &mut symbols)
            .is_empty()
    );
    let later = Expression::new(
        &mut config,
        (
            TempExpr::Path(TempPath {
                segments: vec!["other".into()],
            }),
            span,
        ),
        &mut symbols,
        None,
    );
    assert!(later.is_poisoned());
    assert_eq!(config.error_handle().errors.len(), 1);
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(ResolveError::UnknownVariable),
        span
    )));
}

#[test]
fn null_literal_is_not_poison() {
    let mut config = Config::new(
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let expression = Expression::new(
        &mut config,
        (
            TempExpr::Literal {
                kind: TempLiteralKind::Null,
                text: "null".into(),
            },
            (0..4).into(),
        ),
        &mut symbols,
        None,
    );
    assert!(expression.is_null());
    assert!(!expression.is_poisoned());
    assert!(!config.is_poisoned());
}
