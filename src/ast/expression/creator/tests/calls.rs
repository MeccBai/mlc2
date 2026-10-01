use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::config::Config;
use crate::ast::expression::{Expression, FuncCall};
use crate::ast::function::{FuncSymbol, InterfaceSymbol};
use crate::ast::statement::Variable;
use crate::ast::types::CompileType;
use crate::ast::types::UnitType;
use crate::ast::types::base_type::DataType;
use crate::ast::{EnumBool, SymbolTable};
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::{TempCallee, TempExpr, TempLiteralKind, TempPath};

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

fn path(name: &str) -> TempExpr {
    TempExpr::Path(TempPath {
        segments: vec![name.into()],
    })
}

fn integer(value: &str) -> TempExpr {
    TempExpr::Literal {
        kind: TempLiteralKind::Integer,
        text: value.into(),
    }
}

#[test]
fn plain_function_call_resolves_symbol_arguments_and_return_type() {
    let (mut config, mut symbols) = setup();
    let span = (0..8).into();
    let i32_type = symbols.get_base(DataType::Integer, 32, true);
    let index = symbols.functions.insert(
        "identity".into(),
        FuncSymbol {
            name: "identity".into(),
            params: vec![(i32_type, "value".into())],
            ret_type: Some(i32_type),
            generics: Vec::new(),
            generic_map: HashMap::new(),
            attributes: Vec::new(),
            exported: false,
        },
    );
    let call = TempExpr::Call {
        callee: TempCallee::Expr(Box::new((path("identity"), span))),
        args: vec![(integer("7"), span)],
    };

    let expression = Expression::new(&mut config, (call, span), &mut symbols, None);
    assert!(matches!(
        &expression,
        Expression::FuncCallE(FuncCall {
            func: EnumBool::False(actual),
            args,
        }) if *actual == index && args.len() == 1
    ));
    assert_eq!(
        expression.type_inference(&mut config, &mut symbols),
        i32_type
    );
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn member_call_resolves_interface_and_prepends_owner() {
    let (mut config, mut symbols) = setup();
    let span = (0..10).into();
    let i32_type = symbols.get_base(DataType::Integer, 32, true);
    let mut point = UnitType::empty();
    point.name = "Point".into();
    let point_type = symbols
        .types
        .insert("Point".into(), CompileType::Unit(point));
    symbols.globals.insert(
        "point".into(),
        Rc::new(Variable {
            name: "point".into(),
            var_type: point_type,
            init_val: Box::new(Expression::null()),
        }),
    );
    let index = symbols.interfaces.insert(
        "Point::get".into(),
        InterfaceSymbol {
            public: true,
            has_self: true,
            mutable: false,
            exported: false,
            owner: point_type,
            attributes: Vec::new(),
            generics: Vec::new(),
            generic_map: HashMap::new(),
            name: "get".into(),
            params: vec![(i32_type, "index".into())],
            ret_type: Some(i32_type),
        },
    );
    let member = TempExpr::Member {
        base: Box::new((path("point"), span)),
        indirect: false,
        name: "get".into(),
        name_span: span,
    };
    let call = TempExpr::Call {
        callee: TempCallee::Expr(Box::new((member, span))),
        args: vec![(integer("0"), span)],
    };

    let expression = Expression::new(&mut config, (call, span), &mut symbols, None);
    assert!(matches!(
        &expression,
        Expression::FuncCallE(FuncCall {
            func: EnumBool::True(actual),
            args,
        }) if *actual == index && args.len() == 2
            && matches!(&args[0], Expression::VarValueE(owner) if owner.name == "point")
    ));
    assert_eq!(
        expression.type_inference(&mut config, &mut symbols),
        i32_type
    );
    assert!(config.error_handle().errors.is_empty());
}
