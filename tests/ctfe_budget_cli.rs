use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};
fn project(root: &std::path::Path, budget: &str) {
    fs::write(root.join("Project.toml"),format!("[Project]\nName='budget'\nVersion='0.1.0'\nOutputDir='build'\n[[Project.Targets]]\nName='main'\nEntry='main.m2'\nType='bin'\n[CTFE]\nMaxTime={budget}\n")).unwrap();
}
#[test]
fn project_ctfe_budget_terminates_infinite_evaluation_without_publishing_cache() {
    let root = tempfile::tempdir().unwrap();
    project(root.path(), "0.05");
    fs::write(root.path().join("main.m2"),"const func spin()->i32{while(true){}return 1;}func main()->i32{const result=spin();return result;}").unwrap();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["build", "main", "--json"])
        .output()
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(!output.status.success());
    let doc: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        doc["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"]
                .as_str()
                .unwrap()
                .contains("exceeded MaxTime (0.050s)")),
        "{doc}"
    );
    let host = mlc_builder::paths::HOST_TRIPLET;
    let directory = root.path().join("build/ctfe").join(host);
    for entry in fs::read_dir(directory).unwrap() {
        assert!(!entry.unwrap().path().join("cache.toml").exists());
    }
    fs::write(root.path().join("main.m2"),"const func answer()->i32{return 42;}func main()->i32{const result=answer();return result;}").unwrap();
    project(root.path(), "2.0");
    let output = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["build", "main"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn ctfe_budget_defaults_to_three_seconds_and_rejects_invalid_values() {
    let root = tempfile::tempdir().unwrap();
    project(root.path(), "3.0");
    let parsed = mlc_builder::project::ProjectFile::load(root.path())
        .unwrap()
        .unwrap();
    assert_eq!(parsed.ctfe.max_time, Duration::from_secs(3));
    assert_eq!(
        mlc_builder::project::CtfeConfig::default().max_time,
        Duration::from_secs(3)
    );
    for value in ["0.0", "-1.0", "nan", "inf", "3.0.0"] {
        project(root.path(), value);
        assert!(
            mlc_builder::project::ProjectFile::load(root.path()).is_err(),
            "{value}"
        );
    }
}
