#![cfg(windows)]
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn invoke(root: &Path, installation: &Path, args: &[&str]) -> Output {
    fs::write(root.join("config.toml"), "").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mlc"));
    command
        .current_dir(root)
        .env("MLC", installation)
        .args(args)
        .args(["--config", "config.toml"]);
    if !args.contains(&"--archiver") {
        command.args(["--archiver", "llvm-ar"]);
    }
    command.output().unwrap()
}
fn success(output: Output) {
    assert!(output.status.success(), "{output:?}");
}
#[test]
fn install_and_consume_all_representations() {
    let installation = tempfile::tempdir().unwrap();
    let library = tempfile::tempdir().unwrap();
    let consumer = tempfile::tempdir().unwrap();
    fs::write(library.path().join("Project.toml"),"[Project]\nName='test'\nVersion='1.5.1'\nOutputDir='build'\n[[Project.Targets]]\nName='test_lib'\nEntry='main.m2'\nType='lib'\n").unwrap();
    fs::write(library.path().join("main.m2"),"import helper; export func value()->i32 { return helper::number(); } export func<T> identity(value:T)->T { return value; }").unwrap();
    fs::write(
        library.path().join("helper.m2"),
        "export func number()->i32 { return 37; }",
    )
    .unwrap();
    let installed = invoke(
        library.path(),
        installation.path(),
        &["install", "test_lib", "--json"],
    );
    assert!(installed.status.success(), "{installed:?}");
    let json: serde_json::Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert_eq!(json["command"], "install");
    assert_eq!(json["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(json["artifacts"][0]["kind"], "package");
    assert!(
        installation
            .path()
            .join("packages/test_lib/1.5.1/source/main.m2")
            .is_file()
    );
    assert!(installation.path().join("packages/packages.toml").is_file());
    fs::write(
        consumer.path().join("main.m2"),
        "import test_lib; func main()->i32 { return test_lib::identity(test_lib::value()); }",
    )
    .unwrap();
    for kind in ["Source", "Static", "Shared"] {
        if kind != "Source" {
            success(invoke(
                library.path(),
                installation.path(),
                &[
                    "install",
                    "test_lib",
                    "--package-type",
                    &kind.to_lowercase(),
                ],
            ));
        }
        for shared in [false, true] {
            if kind != "Source" && shared {
                continue;
            }
            let linkage = if kind == "Source" {
                format!("LinkType='{}'\n", if shared { "Shared" } else { "Static" })
            } else {
                String::new()
            };
            fs::write(consumer.path().join("Project.toml"),format!("[Project]\nName='consumer'\nVersion='1.0.0'\nOutputDir='build'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\n[[Project.Targets.Requires]]\nName='test_lib'\nVersion='1.5.1'\nType='{kind}'\n{linkage}")).unwrap();
            let result = invoke(consumer.path(), installation.path(), &["run", "app"]);
            assert_eq!(
                result.status.code(),
                Some(37),
                "{kind} shared={shared}: {result:?}"
            );
        }
    }
}
#[test]
fn failed_build_does_not_publish() {
    let installation = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Project.toml"),"[Project]\nName='bad'\nVersion='1.0.0'\nOutputDir='build'\n[[Project.Targets]]\nName='bad'\nEntry='main.m2'\nType='lib'\n").unwrap();
    fs::write(root.path().join("main.m2"), "pub func broken( {").unwrap();
    assert!(
        !invoke(root.path(), installation.path(), &["install", "bad"])
            .status
            .success()
    );
    assert!(!installation.path().join("packages/packages.toml").exists());
    assert!(
        !installation
            .path()
            .join("packages/bad/1.0.0/source")
            .exists()
    );
}

#[test]
fn transitive_dependencies_and_profiles() {
    let installation = tempfile::tempdir().unwrap();
    let base = tempfile::tempdir().unwrap();
    let wrapper = tempfile::tempdir().unwrap();
    let consumer = tempfile::tempdir().unwrap();
    fn project(root: &Path, name: &str, kind: &str, requires: &str) {
        fs::write(root.join("Project.toml"),format!("[Project]\nName='{name}'\nVersion='1.0.0'\nOutputDir='build'\n[[Project.Targets]]\nName='{name}'\nEntry='main.m2'\nType='{kind}'\n{requires}")).unwrap();
    }
    fn requirement(name: &str, kind: &str) -> String {
        format!("[[Project.Targets.Requires]]\nName='{name}'\nVersion='1.0.0'\nType='{kind}'\n")
    }
    project(base.path(), "base_lib", "lib", "");
    fs::write(
        base.path().join("main.m2"),
        "export func value()->i32 { return 41; }",
    )
    .unwrap();
    project(
        wrapper.path(),
        "wrapper",
        "lib",
        &requirement("base_lib", "Static"),
    );
    fs::write(
        wrapper.path().join("main.m2"),
        "import base_lib; export func value()->i32 { return base_lib::value()+1; }",
    )
    .unwrap();
    project(
        consumer.path(),
        "app",
        "bin",
        &requirement("wrapper", "Source"),
    );
    fs::write(
        consumer.path().join("main.m2"),
        "import wrapper; func main()->i32 { return wrapper::value(); }",
    )
    .unwrap();
    for mode in ["--release", "--debug"] {
        success(invoke(
            base.path(),
            installation.path(),
            &["install", "base_lib", "--package-type", "static", mode],
        ));
        if !installation
            .path()
            .join("packages/wrapper/1.0.0/source")
            .exists()
        {
            success(invoke(
                wrapper.path(),
                installation.path(),
                &["install", "wrapper", mode],
            ));
        }
        let result = invoke(consumer.path(), installation.path(), &["run", "app", mode]);
        assert_eq!(result.status.code(), Some(42), "{result:?}");
    }
    // A transitive dependency is available to its owner, but cannot be imported by the application.
    fs::write(
        consumer.path().join("main.m2"),
        "import base_lib; func main()->i32 { return base_lib::value(); }",
    )
    .unwrap();
    let result = invoke(consumer.path(), installation.path(), &["build", "app"]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("not a direct dependency"),
        "{result:?}"
    );
}

#[test]
fn declaration_only_source_never_creates_an_archive_or_requires_an_archiver() {
    let installation = tempfile::tempdir().unwrap();
    let source = installation
        .path()
        .join("packages/c_std")
        .join(env!("CARGO_PKG_VERSION"))
        .join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        installation.path().join("packages/packages.toml"),
        format!(
            "[[Packages]]\nName='c_std'\nVersion='{}'\n[Packages.Source]\nDeclarationOnly=true\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    fs::write(source.join("lib.h"), "int32_t abs(int32_t value);").unwrap();
    success(invoke(
        installation.path(),
        installation.path(),
        &["setup", "--archiver", "missing-archiver"],
    ));
    let consumer = tempfile::tempdir().unwrap();
    fs::write(consumer.path().join("Project.toml"),format!("[Project]\nName='app'\nVersion='1.0.0'\nOutputDir='build'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\n[[Project.Targets.Requires]]\nName='c_std'\nVersion='{}'\nType='Source'\n",env!("CARGO_PKG_VERSION"))).unwrap();
    fs::write(
        consumer.path().join("main.m2"),
        "import c_std::lib;func main()->i32{return c_std::lib::abs(-9);}",
    )
    .unwrap();
    let result = invoke(
        consumer.path(),
        installation.path(),
        &["run", "app", "--archiver", "missing-archiver"],
    );
    assert_eq!(result.status.code(), Some(9), "{result:?}");
    fn assert_no_archive(root: &Path) {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                assert_no_archive(&path);
            } else {
                assert_ne!(path.file_name().unwrap(), "c_std.lib");
            }
        }
    }
    assert_no_archive(installation.path());
    assert!(!source.join("c_std.m2").exists());
}
