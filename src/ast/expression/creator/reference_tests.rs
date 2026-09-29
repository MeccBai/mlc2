use std::rc::Rc;

use crate::ast::SymbolTable;
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::expression::operators::Operator;
use crate::ast::statement::Variable;
use crate::ast::types::base_type::DataType;
use crate::ast::types::{CompileType, resolve_type};
use crate::error::ErrorHandle;
use crate::parser::out::{TempExpr, TempPath, TempType};

#[test]
fn address_of_preserves_mutability_in_type_identity() {
    let mut config = Config::new(
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let base_type = symbols.get_base(DataType::Integer, 32, true);
    symbols.globals.insert(
        "a".into(),
        Rc::new(Variable {
            name: "a".into(),
            var_type: base_type,
            init_val: Box::new(Expression::null()),
            immutable: false,
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
    let mutable = reference_type(Operator::MutableAddressOf, &mut config, &mut symbols);
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
