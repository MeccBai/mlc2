use super::*;

#[test]
fn resource_types_parse_in_parameters_returns_and_variables() {
    let module = parse_ok("func f(p:res $mut i32) -> res $i32 { var t:res $mut i32 = p; }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("function expected");
    };
    assert_eq!(
        function.symbol.params[0].ty.as_ref().unwrap().0.dump(),
        "res $mut i32"
    );
    assert_eq!(
        function.symbol.return_type.as_ref().unwrap().0.dump(),
        "res $i32"
    );
    for source in ["func f(p:res i32) {}", "func f(p:res res $i32) {}"] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }
}

#[test]
fn else_if_lowers_to_nested_if_scopes_and_preserves_spans() {
    let source = "func main() { if (a) {} else if (b) {} else if (c) {} else {} return; }";
    let module = parse_ok(source);
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    assert_eq!(statements.len(), 2);
    let mut branch = &statements[0];
    for expected in ["a", "b", "c"] {
        let TempStmt::If {
            condition,
            else_scope,
            ..
        } = &branch.0
        else {
            panic!("expected nested if");
        };
        assert_eq!(&source[condition.1.into_range()], expected);
        assert!(source[branch.1.into_range()].starts_with("if ("));
        let scope = else_scope.as_ref().unwrap();
        if expected == "c" {
            assert!(scope.statements.is_empty());
        } else {
            assert_eq!(scope.statements.len(), 1);
            branch = &scope.statements[0];
        }
    }
}

#[test]
fn else_if_without_final_else_is_allowed_but_else_requires_block_or_if() {
    parse_ok("func main() { if (true) {} else if (false) {} }");
    for source in [
        "func main() { if (true) {} else return; }",
        "func main() { if (true) {} else if (false) return; }",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }
}

#[test]
fn local_and_global_variables_share_temp_var_and_for_binding_keeps_span() {
    let source = "global var globalValue:i32 = 1; func main() { var localValue:i32 = 2; for index in [0, 2] {} }";
    let module = parse_ok(source);
    let TempGlobalStmt::Variable(global) = &module[0].0 else {
        panic!("expected global variable");
    };
    assert_eq!(&source[global.name_span.into_range()], "globalValue");

    let TempGlobalStmt::Func(function) = &module[1].0 else {
        panic!("expected function");
    };
    let statements = &function.body.as_ref().unwrap().statements;
    let TempStmt::Variable(local) = &statements[0].0 else {
        panic!("expected local variable");
    };
    assert_eq!(&source[local.name_span.into_range()], "localValue");
    assert_eq!(&source[local.initializer.1.into_range()], "2");

    let TempStmt::For { binding, .. } = &statements[1].0 else {
        panic!("expected for statement");
    };
    assert_eq!(binding.0, "index");
    assert_eq!(&source[binding.1.into_range()], "index");
}

#[test]
fn parses_references_arrays_and_half_open_for() {
    let module = parse_ok(
        "func main() { var a:i32 = 0; var c:$i32 = @a; $c = 10; \
         var values = [1, 2, 3]; for i in [0, 10] { continue; } }",
    );
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    let statements = &function.body.as_ref().unwrap().statements;

    let TempStmt::Variable(out::TempVar {
        ty: Some((ty, _)),
        initializer: (value, _),
        ..
    }) = &statements[1].0
    else {
        panic!("expected a referenced variable");
    };
    assert!(matches!(ty, TempType::Reference { mutable: false, .. }));
    assert!(matches!(
        value,
        TempExpr::Unary {
            op: crate::operators::Operator::AddressOf,
            ..
        }
    ));
    assert!(matches!(
        &statements[2].0,
        TempStmt::Assignment {
            target: (
                TempExpr::Unary {
                    op: crate::operators::Operator::Dereference,
                    ..
                },
                _
            ),
            ..
        }
    ));
    assert!(matches!(
        &statements[3].0,
        TempStmt::Variable(out::TempVar { initializer: (TempExpr::Array(values), _), .. })
            if values.len() == 3
    ));
    assert!(matches!(
        &statements[4].0,
        TempStmt::For { binding, .. } if binding.0 == "i"
    ));
}

#[test]
fn parses_default_match_and_only_explicit_anonymous_blocks() {
    let module = parse_ok("func main() { match (value) { _ => {} } anonymous {} }");
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("expected a function");
    };
    assert!(matches!(
        &function.body.as_ref().unwrap().statements[0].0,
        TempStmt::Match { branches, .. }
            if matches!(branches[0].0, TempMatchPattern::Default)
    ));
    assert!(matches!(
        &function.body.as_ref().unwrap().statements[1].0,
        TempStmt::Anonymous(_)
    ));

    let source = "func main() { {} }";
    let lexed = tokenize(source).unwrap();
    let (_, errors) = parse(&lexed.tokens, source.len());
    assert!(!errors.is_empty());
}

#[test]
fn rejects_deprecated_range_operator() {
    let source = "func main() { for i in 0..10 {} }";
    let lexed = tokenize(source).unwrap();
    let (_, errors) = parse(&lexed.tokens, source.len());
    assert!(!errors.is_empty());
}
