use super::*;

#[test]
fn union_declarations_and_borrowed_type_matches() {
    let module = parse_ok(
        "unit<T> Box{pub value:T;} union<T> V[Box<T>,i32];func f(){var x=V<i32>{1};match(y=@mut x){Box<i32> =>{},i32=>{}}}",
    );
    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!("unit expected")
    };
    assert!(!unit.is_union);
    let TempGlobalStmt::Unit(union) = &module[1].0 else {
        panic!("union expected")
    };
    assert!(union.is_union);
    assert_eq!(union.generics.len(), 1);
    assert_eq!(union.members.len(), 2);
    assert!(
        union
            .members
            .iter()
            .all(|member| member.ty.1.start < member.ty.1.end)
    );
}

#[test]
fn union_rejects_wildcard_and_mixed_generic_positions() {
    for source in [
        "union<T> V<E>[T,E];",
        "union V[i32];func f(){match(y=@x){_=>{}}}",
    ] {
        let tokens = tokenize(source).unwrap();
        assert!(!parse(&tokens.tokens, source.len()).1.is_empty());
    }
    parse_ok("union V<T>[T,i32];");
}
