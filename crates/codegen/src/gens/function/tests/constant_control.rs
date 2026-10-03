use super::*;

#[test]
fn constant_if_promotes_only_selected_scope() {
    let ir = emit(
        "func main() { const enabled = true; if (enabled && 1+2*3 == 7) { var live = 42; } else { var dead = 9; } if (!true) { var other_dead = 1; } }",
        "constant-if",
    );
    assert!(!ir.contains("if.then"), "{ir}");
    assert!(!ir.contains("br i1"), "{ir}");
    assert!(!ir.contains("i32 9"), "{ir}");
    let (ast, package) = parse(
        "func main() { if (true) { var local = 1; } else { var discarded = 2; } var local = 3; }",
    );
    let generated = FunctionGenerator::generate(&ast.body[0], &package);
    let scope_exit = generated
        .exits
        .iter()
        .find(|exit| exit.kind == ExitKind::Scope)
        .unwrap();
    let names: Vec<_> = generated
        .variables
        .between(scope_exit.from, scope_exit.to)
        .iter()
        .map(|variable| variable.name.as_str())
        .collect();
    assert_eq!(names, ["local"]);
    emit(
        "func main() { if (true) { return; } else { var unreachable = 3; } }",
        "constant-if-return",
    );
}

#[test]
fn false_while_and_zero_iteration_for_have_no_loop_ir() {
    let ir = emit(
        "func side() -> i32 { return 8; } func main() { while (2 > 3) { side(); } for i in [5,2] { side(); } for j in [2,2] { side(); } }",
        "empty-loops",
    );
    assert!(!ir.contains("while.condition"), "{ir}");
    assert!(!ir.contains("for.condition"), "{ir}");
    assert!(!ir.contains("call i32 @\"side\""), "{ir}");
    let live = emit(
        "func main() { for i in [0,2] { if (i == 1) { break; } } while (true) { break; } }",
        "live-loops",
    );
    assert!(live.contains("for.condition"), "{live}");
    assert!(live.contains("while.condition"), "{live}");
}

#[test]
fn mutable_local_initializers_are_not_propagated_and_side_effects_are_kept() {
    let ir = emit(
        "func effect() -> bool { return true; } func start() -> i32 { return 2; } func main() { var flag = false; flag = true; if (flag) {} if (effect() || true) {} for i in [start(),0] {} }",
        "dynamic-control",
    );
    assert!(ir.contains("if.then"), "{ir}");
    assert!(ir.contains("call i1 @\"effect\""), "{ir}");
    assert!(ir.contains("call i32 @\"start\""), "{ir}");
    assert!(ir.contains("for.condition"), "{ir}");
    let short = emit(
        "func effect() -> bool { return true; } func main() { if (false && effect()) {} if (true || effect()) {} }",
        "short-control",
    );
    assert!(!short.contains("call i1 @\"effect\""), "{short}");
}

#[test]
fn constant_match_selects_enum_and_default_without_runtime_comparisons() {
    let ir = emit(
        "enum Color { Red, Green } func main() -> i32 { const color = Color::Green; match (color) { _ => { return 9; }, Color::Red => { return 1; }, Color::Green => { return 0; } } }",
        "constant-match",
    );
    assert!(!ir.contains("match.arm"), "{ir}");
    assert!(!ir.contains("icmp"), "{ir}");
    let fallback = emit(
        "func main() -> i32 { match (7) { 1 => { return 1; }, _ => { return 0; } } }",
        "match-default",
    );
    assert!(!fallback.contains("match.arm"), "{fallback}");
    let unmatched = emit(
        "func main() -> i32 { for i in [0,1] { match (8) { 1 => { return 1; } } } return 0; }",
        "match-unmatched",
    );
    assert!(!unmatched.contains("match.arm"), "{unmatched}");
    assert!(unmatched.contains("for.step"), "{unmatched}");
}

#[test]
fn constants_the_evaluator_cannot_expand_keep_runtime_control_flow() {
    let ir = emit(
        "func main() { const number = cast<f32>(1.0); if (number == number) {} match (number) { cast<f32>(1.0) => {}, _ => {} } }",
        "unexpanded-constants",
    );
    assert!(ir.contains("if.then"), "{ir}");
    assert!(ir.contains("match.arm"), "{ir}");
}
