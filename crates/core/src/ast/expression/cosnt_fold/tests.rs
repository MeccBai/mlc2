use super::*;
use crate::ast::{config::FileId, statement::Variable};
use crate::diagnostic::{error::ErrorHandle, warning::WarningHandle};
use std::rc::Rc;

fn config() -> Config {
    Config::new(
        FileId::new(0),
        vec![],
        String::new(),
        String::new(),
        ErrorHandle::new(String::new()),
        WarningHandle::new(String::new()),
    )
}

fn literal(text: &str, symbols: &SymbolTable) -> CompAtom {
    CompAtom::ConstValueA(ConstValue {
        value: text.into(),
        ty: symbols.get_base(DataType::Integer, 32, true),
    })
}

fn expression(values: &[&str], operators: Vec<Operator>, symbols: &SymbolTable) -> Expression {
    Expression::CompositeE(Composite {
        members: values.iter().map(|v| literal(v, symbols)).collect(),
        operators,
    })
}

fn assert_value(expr: Expression, expected: &str, symbols: &SymbolTable) {
    let Expression::ConstValueE(value) = expr.const_fold(&mut config(), symbols) else {
        panic!("expected folded constant");
    };
    assert_eq!(value.value, expected);
}

#[test]
fn precedence_and_left_associativity() {
    let symbols = SymbolTable::new();
    assert_value(
        expression(
            &["1", "2", "3"],
            vec![Operator::Add, Operator::Multiply],
            &symbols,
        ),
        "7",
        &symbols,
    );
    assert_value(
        expression(
            &["20", "3", "2"],
            vec![Operator::Divide, Operator::Remainder],
            &symbols,
        ),
        "0",
        &symbols,
    );
    assert_value(
        expression(
            &["10", "3", "2"],
            vec![Operator::Subtract, Operator::Subtract],
            &symbols,
        ),
        "5",
        &symbols,
    );
}

#[test]
fn recursively_folds_groups() {
    let symbols = SymbolTable::new();
    let group = CompAtom::from_expr(expression(&["1", "2"], vec![Operator::Add], &symbols));
    assert_value(
        Expression::CompositeE(Composite {
            members: vec![group, literal("3", &symbols)],
            operators: vec![Operator::Multiply],
        }),
        "9",
        &symbols,
    );
}

#[test]
fn invalid_arithmetic_and_other_operators_are_preserved() {
    let symbols = SymbolTable::new();
    for (a, op, b) in [
        ("1", Operator::Divide, "0"),
        ("1", Operator::Remainder, "0"),
        ("2147483647", Operator::Add, "1"),
        ("1", Operator::Equal, "1"),
    ] {
        let expr = expression(&[a, b], vec![op], &symbols);
        assert_eq!(expr.clone().const_fold(&mut config(), &symbols), expr);
    }
}

#[test]
fn only_const_variables_are_expanded() {
    let mut symbols = SymbolTable::new();
    let base = symbols.get_base(DataType::Integer, 32, true);
    for state in [ValueType::Flex, ValueType::Final, ValueType::Constant] {
        let variable = Rc::new(Variable {
            read_count: Default::default(),
            declaration_span: (0..0).into(),
            name: "a".into(),
            var_type: base.into(state, &mut symbols.types),
            init_val: Box::new(literal("5", &symbols).to_expression()),
        });
        let expr = Expression::VarValueE(variable);
        if state == ValueType::Constant {
            assert_value(expr, "5", &symbols);
        } else {
            assert_eq!(expr.clone().const_fold(&mut config(), &symbols), expr);
        }
    }
}

#[test]
fn float_arithmetic_retains_type() {
    let symbols = SymbolTable::new();
    for bits in [32, 64] {
        let ty = symbols.get_base(DataType::Float, bits, true);
        let expr = Expression::CompositeE(Composite {
            members: ["5.5", "2.0"]
                .map(|v| {
                    CompAtom::ConstValueA(ConstValue {
                        value: v.into(),
                        ty,
                    })
                })
                .to_vec(),
            operators: vec![Operator::Remainder],
        });
        let Expression::ConstValueE(value) = expr.const_fold(&mut config(), &symbols) else {
            panic!()
        };
        assert_eq!(value.value, "1.5");
        assert_eq!(value.ty, ty);
    }
}

#[test]
fn variable_construction_folds_initializer_once() {
    use crate::parser::out::{TempExpr, TempLiteralKind, TempVar};
    let mut symbols = SymbolTable::new();
    let mut config = config();
    let span = (0..1).into();
    let variable = Variable::new(
        &mut config,
        TempVar {
            name: "a".into(),
            name_span: span,
            ty: None,
            value_type: ValueType::Flex,
            initializer: (
                TempExpr::Binary {
                    operands: ["2", "3"]
                        .map(|text| {
                            (
                                TempExpr::Literal {
                                    kind: TempLiteralKind::Integer,
                                    text: text.into(),
                                },
                                span,
                            )
                        })
                        .to_vec(),
                    operators: vec![Operator::Add],
                },
                span,
            ),
        },
        &mut symbols,
        None,
    );
    assert!(config.error_handle().errors.is_empty());
    let Expression::ConstValueE(value) = &*variable.init_val else {
        panic!("initializer not folded")
    };
    assert_eq!(value.value, "5");
    assert_eq!(
        variable.var_type.value_type(&symbols.types),
        ValueType::Flex
    );
    let reference = Expression::VarValueE(variable);
    assert_eq!(
        reference.clone().const_fold(&mut config, &symbols),
        reference
    );
}
