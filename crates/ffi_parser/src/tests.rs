use super::*;
use mlc_syntax::parser::out::*;
const TARGET: &str = "x86_64-pc-windows-msvc";

#[test]
fn parses_fixed_width_structs_prototypes_and_pointer_constness() {
    let module = parse("struct P { int32_t x; uint8_t bytes[4]; struct P *next; }; void update(struct P *p); int32_t print(const int8_t *pattern, ...); void *create(void);", TARGET).unwrap();
    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!()
    };
    assert_eq!(unit.attributes, ["c_abi"]);
    assert_eq!(unit.members[1].ty.0.dump(), "[u8:4]");
    let TempGlobalStmt::Func(function) = &module[2].0 else {
        panic!()
    };
    assert_eq!(
        function.symbol.params[0].ty.as_ref().unwrap().0.dump(),
        "$i8"
    );
    assert_eq!(function.symbol.params[1].name, "...");
    let TempGlobalStmt::Func(function) = &module[3].0 else {
        panic!()
    };
    assert_eq!(
        function.symbol.return_type.as_ref().unwrap().0.dump(),
        "$mut i8"
    );
    assert!(function.body.is_none());
}

#[test]
fn unsupported_language_features_and_uncertain_types_are_errors() {
    for source in [
        "#include <stdint.h>",
        "#ifndef X",
        "typedef int32_t I;",
        "inline int32_t f(void);",
        "extern int32_t f(void);",
        "int f(void);",
        "char *f(void);",
        "size_t f(void);",
        "long f(void);",
        "struct Opaque;",
        "struct P {};",
        "struct P { uint8_t bits:2; };",
        "struct P {uint8_t bytes[];};",
        "struct P {int32_t a,b;};",
        "int32_t global;",
        "void f(void) {}",
        "void f();",
        "void f(...);",
        "void f(int32_t, ..., int32_t);",
        "void f(int32_t (*callback)(int32_t));",
        "__declspec(dllexport) void f(void);",
        "void f(volatile int32_t *p);",
        "void f(int32_t *restrict p);",
        "struct P {int32_t x;};struct P f(void);",
        "struct P {int32_t x;};void f(struct P p);",
        "void f(struct Unknown *p);",
        "struct P {const int32_t x;};",
        "/* unfinished",
        "void f(void);💥",
    ] {
        assert!(parse(source, TARGET).is_err(), "must reject: {source}");
    }
}

#[test]
fn rejects_duplicates_cycles_and_overflow_without_panicking() {
    for source in [
        "void f(void);void f(void);",
        "struct P{int32_t x;int32_t x;};",
        "void f(int32_t x,int32_t x);",
        "struct P{struct P value;};",
        "struct A{struct B b;};struct B{struct A a;};",
        "struct P{uint64_t a[18446744073709551615];};",
        "struct i32{int32_t x;};",
    ] {
        assert!(parse(source, TARGET).is_err(), "{source}");
    }
    assert!(parse("void f(void);", "i686-pc-windows-msvc").is_err());
}

#[test]
fn comments_unnamed_parameters_and_truncated_inputs() {
    let source = "// 中文\n struct P { uint8_t bytes[2][3]; }; /* explanation */ int32_t f(int32_t, const uint8_t *);";
    let module = parse(source, TARGET).unwrap();
    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!()
    };
    assert_eq!(unit.members[0].ty.0.dump(), "[[u8:3]:2]");
    for (end, _) in source.char_indices() {
        let _ = parse(&source[..end], TARGET);
    }
}

#[test]
fn universal_keeps_explicit_width_rules_and_matches_supported_64_bit_models() {
    let source = "struct Value { int64_t integer; double real; void *pointer; }; int32_t inspect(const struct Value *value);";
    let universal = parse(source, "universal").unwrap();
    for target in [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        assert_eq!(universal, parse(source, target).unwrap());
    }
    assert!(parse("long f(void);", "universal").is_err());
    assert!(parse("int32_t f(void);", "i686-pc-windows-msvc").is_err());
}
