use super::*;
use crate::diagnostic::{error::{ErrorHandle, ErrorInfo}, warning::WarningHandle};

fn config() -> Config {
    Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    )
}

fn path(name: &str) -> TempPath {
    TempPath {
        segments: name.split("::").map(str::to_owned).collect(),
    }
}

#[test]
fn generic_requirement_errors_keep_their_original_span() {
    let cases = [
        (
            "other::generic::is_integer",
            None,
            ConstraintError::InvalidRequirement,
        ),
        (
            "std::generic::unknown",
            None,
            ConstraintError::NoRequirements,
        ),
        ("std::generic::max_bits", None, ConstraintError::NoArgument),
        (
            "std::generic::is_integer",
            Some("1"),
            ConstraintError::InvalidArgument,
        ),
    ];
    for (index, (name, argument, reason)) in cases.into_iter().enumerate() {
        let mut config = config();
        let span: Span = (index * 10..index * 10 + 5).into();
        assert_eq!(
            GenericTypeRequire::new(&mut config, (path(name), argument.map(str::to_owned)), span),
            None
        );
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::Resolve(ResolveError::Constraint(reason)),
            span
        )));
        assert_eq!(config.error_handle().errors.len(), 1);
    }
}

#[test]
fn valid_generic_requirement_does_not_submit_an_error() {
    let mut config = config();
    let span: Span = (4..31).into();
    assert_eq!(
        GenericTypeRequire::new(
            &mut config,
            (path("std::generic::max_bits"), Some("16".into())),
            span,
        ),
        Some(GenericTypeRequire::MaxBits(16))
    );
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn generic_conversion_uses_requirement_span() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let span: Span = (12..34).into();
    let prototype = TempGeneric {
        visibility: Default::default(),
        name: "G".into(),
        name_span: span,
        requirements: vec![(
            TempConstraints::Type {
                path: path("std::generic::unknown"),
                argument: None,
            },
            span,
        )],
        attributes: Vec::new(),
    };

    let (generic, name_span) = GenericRequire::new(&mut config, prototype, &mut symbols);
    assert_eq!(name_span, span);
    assert!(generic.requires.is_empty());
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(ResolveError::Constraint(ConstraintError::NoRequirements)),
        span,
    )));
}
