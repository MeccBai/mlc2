use super::*;

#[test]
fn lowers_pipe_to_free_function_call() {
    let source = "func main() { var result = {1, 2} |> std::sum(3); }";
    let module = parse_ok(source);
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let Some(TempStmt::Variable(out::TempVar {
        initializer: (TempExpr::Call { callee, args }, _),
        ..
    })) = function
        .body
        .as_ref()
        .and_then(|body| body.statements.first())
        .map(|statement| &statement.0)
    else {
        panic!("expected the pipeline to lower to a call");
    };
    assert_eq!(args.len(), 3);
    assert!(matches!(
        callee,
        out::TempCallee::Expr(expr)
            if matches!(&expr.0, TempExpr::Path(TempPath { segments })
                if segments == &["std", "sum"])
    ));
}

#[test]
fn rejects_member_function_pipeline() {
    let source = "func main() { x |> object.f(a); }";
    let lexed = tokenize(source).unwrap();
    let (_, errors) = parse(&lexed.tokens, source.len());
    assert!(!errors.is_empty());
}

#[test]
fn temp_expressions_flatten_type_preserving_operators_across_precedence() {
    use crate::operators::Operator;

    let module = parse_ok("func main() { var x = a + b - c * d; var y = object.field; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert!(matches!(
        &statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Add, Operator::Subtract, Operator::Multiply]
            && operands.len() == 4
    ));
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            initializer: (
                TempExpr::Member {
                    indirect: false,
                    ..
                },
                _
            ),
            ..
        })
    ));
}

#[test]
fn binary_flattening_preserves_parentheses_and_member_access_kind() {
    use crate::operators::Operator;

    let module = parse_ok("func main() { var x = a - (b - c); var y = ptr->field + 1; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert!(matches!(
        &statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Subtract]
            && matches!(&operands[1].0, TempExpr::Group(inner)
                if matches!(&inner.0, TempExpr::Binary { operators, .. }
                    if operators == &[Operator::Subtract]))
    ));
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, .. }, _),
            ..
        }) if matches!(&operands[0].0, TempExpr::Member { indirect: true, .. })
    ));
}

#[test]
fn flattened_binary_keeps_call_init_and_array_operands() {
    let module = parse_ok("func main() { var x = f(1) + Point{2} + [3]; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let TempStmt::Variable(out::TempVar {
        initializer: (TempExpr::Binary { operands, .. }, _),
        ..
    }) = &function.body.as_ref().unwrap().statements[0].0
    else {
        panic!("expected a flattened binary expression");
    };
    assert!(matches!(&operands[0].0, TempExpr::Call { .. }));
    assert!(matches!(&operands[1].0, TempExpr::Init { .. }));
    assert!(matches!(&operands[2].0, TempExpr::Array(_)));
}

#[test]
fn initializer_target_is_a_type_not_an_expression() {
    let source = "func main() { var a = {1, 2}; var b = P{1, 2}; var c = Box<i32>{3}; }";
    let module = parse_ok(source);
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;

    assert!(matches!(
        &statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Init { target: None, .. }, _),
            ..
        })
    ));
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Init { target: Some((TempType::Path(path), span)), .. }, _),
            ..
        }) if path.segments == ["P"] && &source[span.into_range()] == "P"
    ));
    assert!(matches!(
        &statements[2].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Init {
                target: Some((TempType::Generic { base, args }, span)),
                ..
            }, _),
            ..
        }) if base.segments == ["Box"] && args.len() == 1
            && &source[span.into_range()] == "Box<i32>"
    ));
}

#[test]
fn initializer_rejects_an_expression_as_target() {
    for source in [
        "func main() { var x = make_point(){1, 2}; }",
        "func main() { var x = (P){1, 2}; }",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "{source} should be rejected");
    }
}

#[test]
fn comparisons_form_type_changing_binary_boundaries() {
    use crate::operators::Operator;

    let module = parse_ok("func main() { var x = a + b * c < d + e; var y = a & b == c; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert!(matches!(
        &statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Less]
            && matches!(&operands[0].0, TempExpr::Binary { operators, .. }
                if operators == &[Operator::Add, Operator::Multiply])
            && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                if operators == &[Operator::Add])
    ));
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::BitAnd]
            && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                if operators == &[Operator::Equal])
    ));
}

#[test]
fn logical_operators_keep_boolean_boundaries() {
    use crate::operators::Operator;

    let module = parse_ok("func main() { var x = a < b && c < d; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    assert!(matches!(
        &function.body.as_ref().unwrap().statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::LogicalAnd]
            && operands.iter().all(|(operand, _)| matches!(operand,
                TempExpr::Binary { operators, .. } if operators == &[Operator::Less]))
    ));
}

#[test]
fn indexing_is_a_type_changing_binary_subexpression() {
    use crate::operators::Operator;

    let module = parse_ok(
        "func main() { var x = values[i + 1] * 2; var y = matrix[row][column]; var z = [7, 8][0]; }",
    );
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert!(matches!(
        &statements[0].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Multiply]
            && matches!(&operands[0].0, TempExpr::Binary { operands, operators }
                if operators == &[Operator::Index]
                    && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                        if operators == &[Operator::Add]))
    ));
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Index]
            && matches!(&operands[0].0, TempExpr::Binary { operators, .. }
                if operators == &[Operator::Index])
    ));
    assert!(matches!(
        &statements[2].0,
        TempStmt::Variable(out::TempVar {
            initializer: (TempExpr::Binary { operands, operators }, _),
            ..
        }) if operators == &[Operator::Index]
            && matches!(&operands[0].0, TempExpr::Array(values) if values.len() == 2)
    ));
}

#[test]
fn address_of_defaults_to_immutable_and_mut_requires_keyword() {
    use crate::operators::Operator;

    let module = parse_ok("func main() { var a = 1; var x:$i32 = @a; var y:$mut i32 = @mut a; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert!(matches!(
        &statements[1].0,
        TempStmt::Variable(out::TempVar {
            ty: Some((TempType::Reference { mutable: false, .. }, _)),
            initializer: (
                TempExpr::Unary {
                    op: Operator::AddressOf,
                    ..
                },
                _
            ),
            ..
        })
    ));
    assert!(matches!(
        &statements[2].0,
        TempStmt::Variable(out::TempVar {
            ty: Some((TempType::Reference { mutable: true, .. }, _)),
            initializer: (
                TempExpr::Unary {
                    op: Operator::MutOf,
                    ..
                },
                _
            ),
            ..
        })
    ));
}
