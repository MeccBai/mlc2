use std::rc::Rc;

use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::statement::Variable;
use crate::ast::symbols::{EnumBool, SymbolTable};
use crate::ast::types::ValueType;
use crate::ast::types::base_type::DataType;
use crate::diagnostic::error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::{TempExpr, TempLiteralKind, TempPath, TempVar};

#[test]
fn const_requires_constant_initializer() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let i32_type = symbols.get_base(DataType::Integer, 32, true);
    symbols.globals.insert(
        "runtime".into(),
        (
            0,
            Rc::new(Variable {
                name: "runtime".into(),
                var_type: i32_type,
                init_val: Box::new(Expression::null()),
            }),
        ),
    );
    let span = (0..1).into();
    let declaration = |name: &str, initializer| TempVar {
        name: name.into(),
        name_span: span,
        ty: None,
        initializer: (initializer, span),
        value_type: ValueType::Constant,
    };
    let constant = Variable::new(
        &mut config,
        declaration(
            "constant",
            TempExpr::Literal {
                kind: TempLiteralKind::Integer,
                text: "1".into(),
            },
        ),
        &mut symbols,
        None,
    );
    assert_eq!(
        constant.var_type.value_type(&symbols.types),
        ValueType::Constant
    );
    assert!(config.error_handle().errors.is_empty());

    Variable::new(
        &mut config,
        declaration(
            "invalid",
            TempExpr::Path(TempPath {
                segments: vec!["runtime".into()],
            }),
        ),
        &mut symbols,
        None,
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::NonConstantInitializer),
        span,
    )));
}
