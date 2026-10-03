use super::build;
use crate::diagnostic::warning::Warning;

#[test]
fn unused_local_points_to_its_name() {
    let source = "func main() -> i32 { var unused = 10; return 0; }";
    let ast = build(source);
    assert!(!ast.config.is_poisoned());
    let info = ast
        .config
        .warning_handle()
        .warnings
        .iter()
        .find(|info| matches!(&info.warning, Warning::UnusedVariable { name } if name == "unused"))
        .expect("unused local warning");
    assert_eq!(&source[info.span.start..info.span.end], "unused");
}

#[test]
fn constant_folding_preserves_source_usage() {
    let ast = build("func main() -> i32 { const a = 10; var b = a; return b; }");
    assert!(!ast.config.is_poisoned());
    assert!(ast.config.warning_handle().warnings.is_empty());
}

#[test]
fn nested_scope_usage_is_counted_before_warning() {
    let ast =
        build("func main() -> i32 { var a = 10; if (a == 10) { var unused = a; } return a; }");
    assert!(!ast.config.is_poisoned());
    let unused: Vec<_> = ast
        .config
        .warning_handle()
        .warnings
        .iter()
        .filter_map(|info| match &info.warning {
            Warning::UnusedVariable { name } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(unused, ["unused"]);
}
