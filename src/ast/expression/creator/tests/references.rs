use std::rc::Rc;

use crate::ast::SymbolTable;
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::expression::operators::Operator;
use crate::ast::statement::Variable;
use crate::ast::types::base_type::DataType;
use crate::ast::types::{CompileType, ValueType, resolve_type};
use crate::diagnostic::error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::{TempExpr, TempPath, TempType};

#[test]
fn address_of_preserves_mutability_in_type_identity() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let base_type = symbols.get_base(DataType::Integer, 32, true);
    symbols.globals.insert(
        "a".into(),
        Rc::new(Variable {
            name: "a".into(),
            var_type: base_type,
            init_val: Box::new(Expression::null()),
        }),
    );
    let span = (0..1).into();
    let reference_type = |op, config: &mut Config, symbols: &mut SymbolTable| {
        let operand = (
            TempExpr::Path(TempPath {
                segments: vec!["a".into()],
            }),
            span,
        );
        Expression::new(
            config,
            (
                TempExpr::Unary {
                    op,
                    value: Box::new(operand),
                },
                span,
            ),
            symbols,
            None,
        )
        .type_inference(config, symbols)
    };

    let immutable = reference_type(Operator::AddressOf, &mut config, &mut symbols);
    let mutable = reference_type(Operator::MutOf, &mut config, &mut symbols);
    assert_ne!(immutable, mutable);
    assert!(
        matches!(symbols.types.get(immutable), CompileType::Ref(reference)
        if reference.base == base_type && !reference.mut_base)
    );
    assert!(
        matches!(symbols.types.get(mutable), CompileType::Ref(reference)
        if reference.base == base_type && reference.mut_base)
    );
    let declared_reference = |mutable, config: &mut Config, symbols: &mut SymbolTable| {
        resolve_type(
            config,
            (
                TempType::Reference {
                    inner: Box::new((
                        TempType::Path(TempPath {
                            segments: vec!["i32".into()],
                        }),
                        span,
                    )),
                    mutable,
                },
                span,
            ),
            symbols,
            None,
        )
    };
    assert_eq!(
        declared_reference(false, &mut config, &mut symbols),
        Some(immutable)
    );
    assert_eq!(
        declared_reference(true, &mut config, &mut symbols),
        Some(mutable)
    );
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn val_can_only_produce_immutable_reference() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let base = symbols.get_base(DataType::Integer, 32, true);
    let final_type = base.into(ValueType::Final, &mut symbols.types);
    symbols.globals.insert(
        "fixed".into(),
        Rc::new(Variable {
            name: "fixed".into(),
            var_type: final_type,
            init_val: Box::new(Expression::null()),
        }),
    );
    let span = (0..1).into();
    let address = |op, config: &mut Config, symbols: &mut SymbolTable| {
        Expression::new(
            config,
            (
                TempExpr::Unary {
                    op,
                    value: Box::new((
                        TempExpr::Path(TempPath {
                            segments: vec!["fixed".into()],
                        }),
                        span,
                    )),
                },
                span,
            ),
            symbols,
            None,
        )
    };
    let immutable = address(Operator::AddressOf, &mut config, &mut symbols);
    let reference = immutable.type_inference(&mut config, &mut symbols);
    assert!(matches!(
        symbols.types.get(reference),
        CompileType::Ref(reference) if !reference.mut_base && reference.base == base
    ));
    let invalid = address(Operator::MutOf, &mut config, &mut symbols);
    assert!(invalid.is_poisoned());
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::InvalidAssignment),
        span,
    )));
}

#[test]
fn final_binding_does_not_freeze_mutable_reference_target() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let base = symbols.get_base(DataType::Integer, 32, true);
    let reference = base
        .make_ref(&mut symbols.types, true)
        .into(ValueType::Final, &mut symbols.types);
    symbols.globals.insert(
        "reference".into(),
        Rc::new(Variable {
            name: "reference".into(),
            var_type: reference,
            init_val: Box::new(Expression::null()),
        }),
    );
    let span = (0..1).into();
    let dereference = Expression::new(
        &mut config,
        (
            TempExpr::Unary {
                op: Operator::Dereference,
                value: Box::new((
                    TempExpr::Path(TempPath {
                        segments: vec!["reference".into()],
                    }),
                    span,
                )),
            },
            span,
        ),
        &mut symbols,
        None,
    );
    assert!(dereference.assignable(&mut config, &mut symbols));
    assert_eq!(
        dereference
            .type_inference(&mut config, &mut symbols)
            .value_type(&symbols.types),
        ValueType::Flex
    );
}
