use mlc_builder::plan::{BuildPlan, ImportResolver};
use mlc_core::diagnostic::Severity;
#[test]
fn parser_failure_keeps_source_data_until_the_caller_renders_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("main.m2");
    let text = "// 中文\r\nfunc main(){var n=;}";
    std::fs::write(&path, text).unwrap();
    let report = BuildPlan::discover(&path, &ImportResolver { lib_dirs: vec![] }).unwrap_err();
    assert!(!report.diagnostics.is_empty());
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    let source = diagnostic.source.as_ref().unwrap();
    assert_eq!(source.text.as_ref(), text);
    assert_eq!(diagnostic.position().unwrap().0, 2);
    assert!(!report.to_string().contains("-->"));
    std::fs::remove_file(path).unwrap();
    assert!(diagnostic.render().contains("func main(){var n=;}"));
}
#[test]
fn project_configuration_errors_are_source_diagnostics_too() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("Project.toml"), "[Project]\nName = [").unwrap();
    let report = mlc_builder::project::ProjectFile::load(root.path()).unwrap_err();
    assert_eq!(report.diagnostics[0].severity, Severity::Error);
    assert!(
        report.diagnostics[0]
            .source
            .as_ref()
            .unwrap()
            .file
            .ends_with("Project.toml")
    );
    assert!(!report.diagnostics[0].message.contains("-->"));
}
