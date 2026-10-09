use std::rc::Rc;

use crate::ast::SymbolTable;
use crate::ast::config::Config;
use crate::ast::expression::operators::Operator;
use crate::ast::expression::{Access, Expression, UnaryExpr};
use crate::ast::statement::Variable;
use crate::ast::types::ValueType;
use crate::ast::types::base_type::DataType;
use crate::ast::types::{CompileType, ListType};
use crate::diagnostic::error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::{TempExpr, TempLiteralKind, TempPath};

fn setup() -> (Config, SymbolTable) {
    (
        Config::new(
            crate::ast::config::FileId::new(0),
            Vec::new(),
            "test".into(),
            "test".into(),
            ErrorHandle::new("test".into()),
            WarningHandle::new("test".into()),
        ),
        SymbolTable::new(),
    )
}

fn integer(value: &str) -> TempExpr {
    TempExpr::Literal {
        kind: TempLiteralKind::Integer,
        text: value.into(),
    }
}

fn index(base: TempExpr, offset: TempExpr) -> TempExpr {
    let span = (0..1).into();
    TempExpr::Binary {
        operands: vec![(base, span), (offset, span)],
        operators: vec![Operator::Index],
    }
}

#[test]
fn reference_index_uses_pointee_mutability_not_binding_mutability() {
    for mutable in [false, true] {
        let (mut config, mut symbols) = setup();
        let element = symbols.get_base(DataType::Integer, 32, true);
        let reference = crate::ast::types::ref_type::RefType::new(element, 1, mutable);
        let ty = symbols.types.insert(
            reference.format(&symbols.types),
            CompileType::Ref(reference),
        );
        let ty = ty.into(ValueType::Final, &mut symbols.types);
        symbols.globals.insert(
            "pointer".into(),
            (
                0,
                Rc::new(Variable {
                    read_count: Default::default(),
                    declaration_span: (0..0).into(),
                    name: "pointer".into(),
                    var_type: ty,
                    init_val: Box::new(Expression::null()),
                }),
            ),
        );
        let expr = Expression::new(
            &mut config,
            (
                index(
                    TempExpr::Path(TempPath {
                        segments: vec!["pointer".into()],
                    }),
                    integer("1"),
                ),
                (0..1).into(),
            ),
            &mut symbols,
            None,
        );
        let inferred = expr.type_inference(&mut config, &mut symbols);
        assert_eq!(
            inferred.value_type(&symbols),
            if mutable {
                ValueType::Flex
            } else {
                ValueType::Final
            }
        );
        assert_eq!(expr.assignable(&mut config, &mut symbols), mutable);
        assert!(config.error_handle().errors.is_empty());
    }
}

#[test]
fn list_index_becomes_unary_access_with_element_type() {
    let (mut config, mut symbols) = setup();
    let element_type = symbols.get_base(DataType::Integer, 32, true);
    let list = ListType::new(element_type, 3);
    let name = list.format(&symbols.types);
    let list_type = symbols.types.insert(name, CompileType::List(list));
    symbols.globals.insert(
        "values".into(),
        (
            0,
            Rc::new(Variable {
                read_count: Default::default(),
                declaration_span: (0..0).into(),
                name: "values".into(),
                var_type: list_type,
                init_val: Box::new(Expression::null()),
            }),
        ),
    );
    let base = TempExpr::Path(TempPath {
        segments: vec!["values".into()],
    });
    let span = (0..1).into();
    let expression = Expression::new(
        &mut config,
        (index(base, integer("0")), span),
        &mut symbols,
        None,
    );

    assert!(matches!(
        &expression,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { .. }))
    ));
    assert_eq!(
        expression.type_inference(&mut config, &mut symbols),
        element_type
    );
    assert!(expression.assignable(&mut config, &mut symbols));
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn array_literal_can_be_indexed() {
    let (mut config, mut symbols) = setup();
    let span = (0..1).into();
    let array = TempExpr::Array(vec![(integer("7"), span), (integer("8"), span)]);
    let expression = Expression::new(
        &mut config,
        (index(array, integer("0")), span),
        &mut symbols,
        None,
    );
    assert!(matches!(
        &expression,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { .. }))
    ));
    assert_eq!(
        expression.type_inference(&mut config, &mut symbols),
        symbols
            .get_base(DataType::Integer, 32, true)
            .into(ValueType::Constant, &mut symbols.types)
    );
    assert!(!expression.assignable(&mut config, &mut symbols));
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn non_integer_index_reports_error() {
    let (mut config, mut symbols) = setup();
    let span = (0..1).into();
    let array = TempExpr::Array(vec![(integer("7"), span)]);
    let boolean = TempExpr::Literal {
        kind: TempLiteralKind::Boolean,
        text: "true".into(),
    };
    Expression::new(
        &mut config,
        (index(array, boolean), span),
        &mut symbols,
        None,
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::InvalidIndexAccess),
        span,
    )));
}
