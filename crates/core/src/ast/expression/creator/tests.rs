use super::*;
use crate::diagnostic::error::ResolveError::UnknownFunction;
use crate::diagnostic::error::{ErrorHandle, ErrorInfo};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::TempLiteralKind;

fn setup() -> (Config, SymbolTable) {
    let config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    (config, SymbolTable::new())
}

#[test]
fn empty_array_reports_missing_type() {
    let (mut config, mut symbols) = setup();
    let span = (0..2).into();
    Expression::new(&mut config, (Array(Vec::new()), span), &mut symbols, None);
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(ResolveError::MissingType),
        span,
    )));
}

#[test]
fn dereferencing_non_reference_reports_error() {
    let (mut config, mut symbols) = setup();
    let span = (0..2).into();
    let value = (
        Literal {
            kind: TempLiteralKind::Integer,
            text: "1".into(),
        },
        (1..2).into(),
    );
    Expression::new(
        &mut config,
        (
            Unary {
                op: Operator::Dereference,
                value: Box::new(value),
            },
            span,
        ),
        &mut symbols,
        None,
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::InvalidDereference),
        span,
    )));
}

#[test]
fn unknown_generic_callee_reports_unknown_function() {
    let (mut config, mut symbols) = setup();
    let span = (0..8).into();
    let callee = TempCallee::GenericPath {
        path: crate::parser::out::TempPath {
            segments: vec!["identity".into()],
        },
        args: vec![(
            crate::parser::out::TempType::Path(crate::parser::out::TempPath {
                segments: vec!["i32".into()],
            }),
            span,
        )],
    };
    Expression::new(
        &mut config,
        (
            Call {
                callee,
                args: Vec::new(),
            },
            span,
        ),
        &mut symbols,
        None,
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(UnknownFunction),
        span,
    )));
}

#[test]
fn registered_generic_function_is_found_in_generic_table() {
    let (mut config, mut symbols) = setup();
    let span = (0..8).into();
    let requirement = symbols.generics.requires.insert(
        "any".into(),
        crate::ast::generic::GenericRequire::empty("any".into()),
    );
    let index = symbols.generics.functions.insert(
        "identity".into(),
        crate::ast::function::FuncSymbol {
            name: "identity".into(),
            params: Vec::new(),
            ret_type: None,
            generics: vec!["T".into()],
            generic_map: HashMap::from([("T".into(), requirement)]),
            attributes: Default::default(),
            exported: false,
        },
    );
    let result = Expression::search_generic_function(
        &mut config,
        (
            Path(crate::parser::out::TempPath {
                segments: vec!["identity".into()],
            }),
            span,
        ),
        &mut symbols,
        None,
    );
    assert_eq!(result, Some(EnumBool::False(index)));
    assert!(config.error_handle().errors.is_empty());
}

mod access;
mod calls;
mod poison;
mod references;
mod visibility;
