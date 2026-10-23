use mlc_builder::artifacts;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn invoke(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn header_generates_targeted_symbols_and_imports_raw_c_names() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("native.h"), "struct Point { int32_t x; int32_t y; }; int32_t abs(int32_t value); void update(struct Point *point);").unwrap();
    success(&invoke(root.path(), &["symbols", "native.h"]));
    let artifact = artifacts::load(&root.path().join("native.sym")).unwrap();
    assert_eq!(artifact.manifest.config.module_name, "native");
    assert_eq!(
        artifact.manifest.config.target,
        mlc_builder::paths::HOST_TRIPLET
    );
    assert_eq!(artifact.manifest.config.object, artifacts::ObjectMode::None);
    assert_eq!(artifact.manifest.symbols.units[0].members.len(), 2);
    fs::write(
        root.path().join("app.m2"),
        "import native;func main()->i32{return native::abs(-7);}",
    )
    .unwrap();
    success(&invoke(root.path(), &["ll", "app.m2", "-o", "ir"]));
    let ir = fs::read_to_string(
        root.path()
            .join("ir/ll/default")
            .join(mlc_builder::paths::HOST_TRIPLET)
            .join("app.ll"),
    )
    .unwrap();
    assert!(ir.contains("@\"abs\""), "{ir}");
    assert!(!ir.contains("@\"native::abs\""), "{ir}");
}

#[test]
fn unsupported_header_does_not_replace_previous_symbols() {
    let root = tempfile::tempdir().unwrap();
    let header = root.path().join("native.h");
    fs::write(&header, "void f(void);").unwrap();
    success(&invoke(
        root.path(),
        &["symbols", "native.h", "--module", "c_std::native"],
    ));
    let original = fs::read(root.path().join("native.sym")).unwrap();
    fs::write(&header, "int f(void);").unwrap();
    let output = invoke(root.path(), &["symbols", "native.h"]);
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains("fixed-width") && error.contains("native.h:1:"),
        "{error}"
    );
    assert_eq!(fs::read(root.path().join("native.sym")).unwrap(), original);
}

#[test]
fn source_declarations_are_rejected_but_c_abi_definitions_remain_valid() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("app.m2");
    fs::write(&input, "#[c_abi]# func abs(value:i32)->i32;func main(){}").unwrap();
    let output = invoke(root.path(), &["ll", "app.m2"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("imported from .sym"));
    fs::write(
        &input,
        "#[c_abi]# export func twice(value:i32)->i32{return value*2;}",
    )
    .unwrap();
    success(&invoke(root.path(), &["ll", "app.m2", "--type", "static"]));
}

#[test]
fn declarations_cannot_be_used_for_another_target() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("native.h"), "void f(void);").unwrap();
    success(&invoke(
        root.path(),
        &[
            "symbols",
            "native.h",
            "--triplet",
            "aarch64-unknown-linux-gnu",
        ],
    ));
    fs::write(root.path().join("app.m2"), "import native;func main(){}").unwrap();
    let output = invoke(
        root.path(),
        &["ll", "app.m2", "--triplet", "x86_64-pc-windows-msvc"],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("declaration target"));
}

#[test]
fn universal_fixed_width_headers_import_across_supported_targets() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("native.h"),
        "int32_t atoi(const int8_t *text);",
    )
    .unwrap();
    success(&invoke(
        root.path(),
        &["symbols", "native.h", "--triplet", "universal"],
    ));
    let artifact = artifacts::load(&root.path().join("native.sym")).unwrap();
    assert!(artifact.manifest.config.target.is_empty());
    fs::write(
        root.path().join("main.m2"),
        "import native;func main()->i32{return native::atoi(const_c_str(\"42\"));}",
    )
    .unwrap();
    for target in [
        "x86_64-pc-windows-msvc",
        "x86_64-pc-windows-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        success(&invoke(
            root.path(),
            &["ll", "main.m2", "--triplet", target],
        ));
    }
}
