use super::*;

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
