use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
};
fn invoke(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap()
}
fn document(output: &Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(!text.contains('\u{1b}'), "{text}");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"))
}
#[test]
fn json_reports_lexer_parser_semantic_and_missing_file_errors() {
    let root = tempfile::tempdir().unwrap();
    for source in [
        "func main(){var n=;}",
        "func main(){var n=unknown;}",
        "func main(){ \u{0001} }",
    ] {
        fs::write(root.path().join("main.m2"), source).unwrap();
        let output = invoke(root.path(), &["ll", "main.m2", "--json"]);
        assert!(!output.status.success());
        let doc = document(&output);
        assert_eq!(doc["success"], false);
        assert_eq!(doc["command"], "ll");
        let diagnostic = &doc["diagnostics"][0];
        assert_eq!(diagnostic["severity"], "error");
        assert!(diagnostic["file"].as_str().unwrap().ends_with("main.m2"));
        assert!(diagnostic["span"]["start"].is_number());
        assert!(!diagnostic["message"].as_str().unwrap().contains("-->"));
    }
    let output = invoke(root.path(), &["--json", "build", "missing.m2"]);
    let doc = document(&output);
    assert!(!output.status.success());
    assert!(
        doc["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("missing.m2")
    );
}
#[test]
fn json_reports_ir_artifacts_warnings_and_formatter_results() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        "func main()->i32{var unused=3;return 0;}",
    )
    .unwrap();
    let output = invoke(root.path(), &["--json", "ll", "main.m2"]);
    assert!(output.status.success());
    let doc = document(&output);
    assert_eq!(doc["success"], true);
    assert_eq!(doc["artifacts"][0]["kind"], "llvm-ir");
    assert!(std::path::Path::new(doc["artifacts"][0]["path"].as_str().unwrap()).is_file());
    assert!(
        doc["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["severity"] == "warning")
    );
    let output = invoke(root.path(), &["fmt", "main.m2", "--json"]);
    assert!(output.status.success());
    assert_eq!(document(&output)["command"], "fmt");
    fs::write(root.path().join("main.m2"), "func main(){var n=;}").unwrap();
    let output = invoke(root.path(), &["fmt", "main.m2", "--json"]);
    assert!(!output.status.success());
    assert_eq!(document(&output)["diagnostics"][0]["severity"], "error");
}
#[test]
fn json_reports_invalid_cli_arguments_and_other_commands() {
    let root = tempfile::tempdir().unwrap();
    let output = invoke(root.path(), &["build", "--json", "--unknown-option"]);
    assert!(!output.status.success());
    assert_eq!(document(&output)["success"], false);
    let output = invoke(root.path(), &["init", "sample", "--json"]);
    assert!(output.status.success());
    assert_eq!(document(&output)["command"], "init");
}
#[test]
#[cfg(windows)]
fn json_build_and_run_keep_program_output_inside_the_document() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("native.h"),
        "int32_t putchar(int32_t value);",
    )
    .unwrap();
    let output = invoke(root.path(), &["symbols", "native.h", "--json"]);
    assert!(output.status.success());
    assert_eq!(document(&output)["artifacts"][0]["kind"], "symbols");
    fs::write(
        root.path().join("main.m2"),
        "import native;func main()->i32{native::putchar(65);return 7;}",
    )
    .unwrap();
    let output = invoke(root.path(), &["build", "main.m2", "--json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let doc = document(&output);
    assert_eq!(doc["command"], "build");
    assert_eq!(doc["artifacts"][0]["kind"], "linked");
    let output = invoke(root.path(), &["run", "main.m2", "--json"]);
    let doc = document(&output);
    assert_eq!(doc["run"]["stdout"], "A");
    assert_eq!(doc["run"]["exit_code"], 7);
    assert_eq!(doc["exit_code"], 7);
    assert_eq!(doc["success"], false);
}

#[test]
fn human_status_colors_respect_no_color() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), "func main()->i32{return 0;}").unwrap();
    let forced = invoke(root.path(), &["ll", "main.m2"]);
    assert!(forced.status.success());
    assert!(String::from_utf8_lossy(&forced.stdout).contains("\u{1b}["));
    let plain = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2"])
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .output()
        .unwrap();
    assert!(plain.status.success());
    assert!(!String::from_utf8_lossy(&plain.stdout).contains('\u{1b}'));
}
