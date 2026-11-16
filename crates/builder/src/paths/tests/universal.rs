use super::*;
#[test]
fn obsolete_installation_directories_are_not_searched() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("lib").join(HOST_TRIPLET)).unwrap();
    fs::create_dir_all(root.path().join("objects/universal")).unwrap();
    fs::write(
        root.path().join("lib").join(HOST_TRIPLET).join("std.m2"),
        "export func old(){}",
    )
    .unwrap();
    fs::write(root.path().join("main.m2"), "import std;func main(){}").unwrap();
    let mut paths = PathResolver::standalone(
        &root.path().join("mlc.exe"),
        root.path(),
        &GlobalConfig::default(),
    )
    .unwrap();
    paths.package_root = root.path().to_owned();
    paths.resolve_packages().unwrap();
    let error = BuildPlan::discover_with_paths(&root.path().join("main.m2"), &paths)
        .unwrap_err()
        .to_string();
    assert!(error.contains("import std not found"), "{error}");
    assert!(!error.contains("objects/universal"), "{error}");
}
#[test]
fn package_declarations_use_portable_inputs_and_target_overrides() {
    use crate::packages::*;
    let root = tempfile::tempdir().unwrap();
    let source = root
        .path()
        .join("packages/c_std")
        .join(env!("CARGO_PKG_VERSION"))
        .join("source");
    fs::create_dir_all(source.join("targets").join(HOST_TRIPLET)).unwrap();
    fs::write(source.join("io.h"), "int32_t portable(void);").unwrap();
    fs::write(
        source.join("targets").join(HOST_TRIPLET).join("io.h"),
        "int32_t targeted(void);",
    )
    .unwrap();
    let registry = Registry {
        packages: vec![Package {
            name: "c_std".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            requires: vec![],
            source: Some(Source {
                entry: None,
                triplet: None,
                declaration_only: true,
            }),
            debug: None,
            release: None,
        }],
    };
    registry.save(root.path()).unwrap();
    let mut paths = PathResolver::standalone(
        &root.path().join("mlc.exe"),
        root.path(),
        &GlobalConfig::default(),
    )
    .unwrap();
    paths.package_root = root.path().to_owned();
    paths.resolve_packages().unwrap();
    fs::write(
        root.path().join("main.m2"),
        "import c_std::io;func main(){}",
    )
    .unwrap();
    assert!(
        BuildPlan::discover_with_paths(&root.path().join("main.m2"), &paths)
            .unwrap_err()
            .to_string()
            .contains("mlc setup")
    );
    prepare_declarations(&paths.toolchain_packages[0], HOST_TRIPLET).unwrap();
    let plan = BuildPlan::discover_with_paths(&root.path().join("main.m2"), &paths).unwrap();
    let declaration = plan
        .targets
        .iter()
        .find(|t| t.module_name == "c_std::io")
        .unwrap();
    let artifact = crate::artifacts::load(&declaration.file).unwrap();
    assert_eq!(artifact.manifest.symbols.functions[0].name, "targeted");
    assert_eq!(artifact.manifest.config.target, HOST_TRIPLET);
}
