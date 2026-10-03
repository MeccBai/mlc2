use super::*;
mod binary;
mod modes;

fn parse(source: &str) -> TempModule {
    let tokens = mlc_syntax::lexer::tokenize(source).unwrap();
    let (module, errors) = mlc_syntax::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    module.unwrap()
}

#[test]
fn positions_bodies_and_private_functions_do_not_change_public_signatures() {
    let a = Symbols::from_module(&parse("export func f(x:i32) -> i32 { return x; }"));
    let b = Symbols::from_module(&parse(
        "\n\nfunc private() {}\nexport func f(x:i32) -> i32 { return x+1; }",
    ));
    assert_eq!(a.public_hash().unwrap(), b.public_hash().unwrap());
    let changed = Symbols::from_module(&parse("export func f(x:i64) -> i32 { return 1; }"));
    assert_ne!(a.public_hash().unwrap(), changed.public_hash().unwrap());
}

#[test]
fn generic_hash_ignores_positions_but_keeps_private_helpers() {
    let a = parse("func<T> f(x:T) -> T { return x; }");
    let b = parse("\n\nfunc<T> f(x:T) -> T { return x; }\n");
    assert_eq!(semantic_hash(&a).unwrap(), semantic_hash(&b).unwrap());
    let changed = parse("func<T> f(x:T) -> T { return x; } func helper() {}");
    assert_ne!(semantic_hash(&a).unwrap(), semantic_hash(&changed).unwrap());
}

#[test]
fn declaration_order_does_not_change_signature_hash() {
    let a = Symbols::from_module(&parse(
        "export func a() {} export func b() {} unit P { a:i32; }; unit Q { b:i64; };",
    ));
    let b = Symbols::from_module(&parse(
        "unit Q { b:i64; }; export func b() {} unit P { a:i32; }; export func a() {}",
    ));
    assert_eq!(a.public_hash().unwrap(), b.public_hash().unwrap());
}
