use super::*;

#[test]
fn generated_unit_destructors_are_recursive_public_and_emitted_once() {
    let ir = emit(
        "unit Inner { p:res $mut i32; }; unit Outer { inner:Inner; }; func consume(x:Outer) {} func f(p:res $mut i32) { var x = Outer{Inner{p}}; consume(x); }",
        "unit-deconstruct",
    );
    assert_eq!(ir.matches("define linkonce_odr void").count(), 2);
    assert!(ir.contains("Outer::deconstruct"));
    assert!(ir.contains("Inner::deconstruct"));
    assert!(ir.contains("zeroinitializer"));
    assert!(ir.contains("call void @\"free\""));
}

#[test]
fn forward_unit_members_and_generic_instances_receive_complete_destructors() {
    let ir = emit(
        "unit Outer { first:res $mut i32; inner:Inner; }; unit Inner { p:res $mut i32; }; unit Box<T> { value:T; }; func f(a:res $mut i32,b:res $mut i32,c:res $mut i32) { var x = Outer{a,Inner{b}}; var y = Box<res $mut i32>{c}; }",
        "forward-generic-deconstruct",
    );
    assert_eq!(ir.matches("define linkonce_odr void").count(), 3);
    let outer = ir
        .split("define linkonce_odr void ")
        .find(|body| {
            body.lines()
                .next()
                .is_some_and(|line| line.contains("Outer::deconstruct"))
        })
        .unwrap()
        .split("}\n")
        .next()
        .unwrap();
    assert!(outer.contains("Inner::deconstruct"));
    assert!(outer.contains("call void @\"free\""));
}

#[test]
fn resource_arrays_and_array_members_are_destroyed_recursively() {
    let ir = emit(
        "unit Outer { items:[Inner:2]; }; unit Inner { p:res $mut i32; }; func f(a:res $mut i32,b:res $mut i32) { var x = Outer{[Inner{a},Inner{b}]}; var moved:res = x; }",
        "array-deconstruct",
    );
    assert_eq!(ir.matches("define linkonce_odr void").count(), 2);
    assert!(ir.contains("i64 1"));
    assert!(ir.contains("i64 0"));
}
