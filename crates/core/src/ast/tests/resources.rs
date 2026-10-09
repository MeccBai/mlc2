use super::*;

#[test]
fn resource_units_move_recursively_and_cannot_move_members() {
    valid(
        "unit Inner { p:res $mut i32; }; unit Outer { inner:Inner; }; func consume(x:Outer) {} func f(p:res $mut i32) { var x = Outer{Inner{p}}; consume(x); }",
    );
    for source in [
        "unit U { pub p:res $mut i32; }; func f(p:res $mut i32) { var x = U{p}; var q:res = x.p; }",
        "unit U { p:res $mut i32; }; func consume(x:U) {} func f(x:U) { consume(x); consume(x); }",
        "unit U { p:res $mut i32; }; func f(x:U) { var y = @x; var z = x; }",
        "unit U {}; pub U::func deconstruct(self) {}",
    ] {
        assert!(build(source).config.is_poisoned(), "{source}");
    }
}

#[test]
fn bare_res_infers_owned_type_and_still_requires_a_resource() {
    valid("func f(p:res $mut i32) { var q:res = p; q[0] = 1; }");
    for source in [
        "func f(p:$mut i32) { var q:res = p; }",
        "func f() { var q:res = 1; }",
        "func f(p:res) {}",
        "func f() -> res {}",
        "func f(p:res $mut i32) { var q:res = p; p[0] = 1; }",
    ] {
        assert!(build(source).config.is_poisoned(), "{source}");
    }
}

#[test]
fn resource_moves_invalidate_sources_and_branches_merge_conservatively() {
    valid(
        "func consume(p:res $mut i32) {} func f(p:res $mut i32, flag:bool) { if (flag) { consume(p); } else { $p = 1; } }",
    );
    for source in [
        "func f(p:res $mut i32) { var q:res $mut i32 = p; $p = 1; }",
        "func consume(p:res $mut i32) {} func f(p:res $mut i32, flag:bool) { if (flag) { consume(p); } else {} $p = 1; }",
        "func consume(p:res $mut i32) {} func f(p:res $mut i32) { match (1) { 1 => { consume(p); }, _ => {} } $p = 1; }",
    ] {
        let ast = build(source);
        assert!(
            ast.config
                .error_handle()
                .render(source)
                .contains("has been moved or destroyed")
        );
    }
}

#[test]
fn borrow_cannot_move_or_outlive_owner_and_plain_ref_cannot_become_res() {
    valid("func f(p:res $mut i32) { var q = p; val value = $q; }");
    for source in [
        "func f(p:res $mut i32) { var q = p; var r:res $mut i32 = p; }",
        "func f(p:res $mut i32) -> $i32 { var q = p; return q; }",
        "func f(p:$mut i32) { var q:res $mut i32 = p; }",
        "func consume(a:$i32, b:res $mut i32) {} func f(p:res $mut i32) { consume(p,p); }",
    ] {
        let ast = build(source);
        assert!(ast.config.is_poisoned(), "{source}");
    }
}

#[test]
fn local_resources_can_move_in_loops_but_outer_resources_cannot() {
    valid("func f(p:res $mut i32) -> res $mut i32 { return p; }");
    let source =
        "func consume(p:res $mut i32) {} func f(p:res $mut i32) { while (true) { consume(p); } }";
    let ast = build(source);
    assert!(
        ast.config
            .error_handle()
            .render(source)
            .contains("repeating loop")
    );
}
