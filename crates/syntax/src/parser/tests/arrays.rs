use super::*;

#[test]
fn fixed_array_type_retains_element_length_and_span() {
    let source = "func main() { var text:[i8:20] = \"hello world\"; var matrix:[[i32:2]:3] = [[1,2],[3,4],[5,6]]; }";
    let module = parse_ok(source);
    let TempGlobalStmt::Func(function) = &module[0].0 else {
        panic!("function expected")
    };
    let TempStmt::Variable(variable) = &function.body.as_ref().unwrap().statements[0].0 else {
        panic!("variable expected")
    };
    let (ty, span) = variable.ty.as_ref().unwrap();
    let TempType::Array { element, length } = ty else {
        panic!("array type expected")
    };
    assert_eq!(*length, 20);
    assert_eq!(element.0.dump(), "i8");
    assert_eq!(&source[span.into_range()], "[i8:20]");
    assert!(crate::serialization::to_toml(ty).is_ok());
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Document {
        module: TempModule,
    }
    let document = Document { module };
    let encoded = crate::serialization::to_toml(&document).unwrap();
    assert_eq!(
        crate::serialization::from_toml::<Document>(&encoded).unwrap(),
        document
    );
}

#[test]
fn invalid_or_overflowed_array_lengths_report_parser_errors() {
    for length in [
        "-1",
        "N",
        "1.5",
        "999999999999999999999999999999999999999999",
    ] {
        let source = format!("func main() {{ var x:[i8:{length}] = \"x\"; }}");
        let lexed = tokenize(&source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "{length}");
    }
}
