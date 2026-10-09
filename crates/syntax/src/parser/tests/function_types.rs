use super::*;

#[test]
fn function_types_are_recursive_and_keep_spans() {
    let module = parse_ok(
        "unit<T> Holder{callback:func(T,$mut T)->T; list:[func():2];} func apply(f:func(func(i32)->i32)->func(i32)->i32){} func main(){var f:func(i32,...)=std::function(print);}",
    );
    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!("unit expected")
    };
    let (
        TempType::Function {
            params,
            returns,
            variadic,
        },
        span,
    ) = &unit.members[0].ty
    else {
        panic!("function type expected")
    };
    assert_eq!(params.len(), 2);
    assert!(returns.is_some());
    assert!(!variadic);
    assert!(
        params
            .iter()
            .all(|(_, inner)| inner.start >= span.start && inner.end <= span.end)
    );
    assert_eq!(unit.members[0].ty.0.dump(), "func(T, $mut T) -> T");
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Document {
        module: TempModule,
    }
    let encoded = crate::serialization::to_toml(&Document {
        module: module.clone(),
    })
    .unwrap();
    assert_eq!(
        crate::serialization::from_toml::<Document>(&encoded)
            .unwrap()
            .module,
        module
    );
}

#[test]
fn malformed_function_types_are_syntax_errors() {
    for source in [
        "unit U{f:func(...,i32);}",
        "unit U{f:func(...,...);}",
        "unit U{f:func(x:i32)->i32;}",
        "unit U{f:func(i32)->;}",
    ] {
        let tokens = tokenize(source).unwrap();
        assert!(
            !parse(&tokens.tokens, source.len()).1.is_empty(),
            "{source}"
        );
    }
}
