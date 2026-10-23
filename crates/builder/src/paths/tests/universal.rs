use super::*;
use crate::artifacts::{self, ObjectMode};
use mlc_core::{
    ast::{
        AbstractSyntaxTree,
        config::{Config, FileId},
        symbols::PackageSymbolTable,
    },
    diagnostic::{error::ErrorHandle, warning::WarningHandle},
};

const IO: &str = include_str!("../../../../../lib/universal/c_std/io.h");

fn fixture() -> (tempfile::TempDir, PathResolver, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let paths = PathResolver::new(
        &root.path().join("mlc.exe"),
        root.path(),
        None,
        &GlobalConfig::default(),
    )
    .unwrap();
    let entry = root.path().join("main.m2");
    fs::write(&entry, "import c_std::io; func main() {}").unwrap();
    let universal = root.path().join("objects/universal/c_std");
    fs::create_dir_all(&universal).unwrap();
    let input = universal.join("io.h");
    fs::write(&input, IO).unwrap();
    artifacts::convert_c_header(&input, &universal.join("io.sym"), "c_std::io", "universal")
        .unwrap();
    (root, paths, entry)
}

#[test]
fn universal_printf_is_a_valid_exported_c_declaration() {
    let (_root, paths, entry) = fixture();
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    let artifact = artifacts::load(&plan.targets[1].file).unwrap();
    assert_eq!(artifact.manifest.config.object, ObjectMode::None);
    let symbol = artifact
        .manifest
        .symbols
        .functions
        .iter()
        .find(|function| function.name == "printf")
        .unwrap();
    assert_eq!(symbol.name, "printf");
    assert_eq!(symbol.params[0].ty.as_ref().unwrap().0.dump(), "$i8");
    assert_eq!(symbol.params[1].name, "...");
    assert_eq!(symbol.attributes, ["c_abi"]);
    let config = Config::new(
        FileId::new(0),
        vec![],
        "c_std::io".into(),
        "io.h".into(),
        ErrorHandle::new("io".into()),
        WarningHandle::new("io".into()),
    );
    let mut ast = AbstractSyntaxTree::new(config, artifact.manifest.symbols.declarations());
    let mut package = PackageSymbolTable::new();
    ast.export(&mut package);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
}

#[test]
fn target_library_wins_over_universal() {
    let (_root, paths, entry) = fixture();
    let target = paths.imports.lib_dirs[0].join("c_std");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("io.m2"), "export func platform() {}").unwrap();
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert_eq!(
        plan.targets[1].file,
        target.join("io.m2").canonicalize().unwrap()
    );
}

#[test]
fn all_target_directories_precede_universal_fallback() {
    let (root, mut paths, entry) = fixture();
    let second = root.path().join("other-lib").join(HOST_TRIPLET);
    fs::create_dir_all(second.join("c_std")).unwrap();
    fs::write(second.join("c_std/io.m2"), "export func platform() {}").unwrap();
    paths.imports.lib_dirs.push(second.clone());
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert_eq!(
        plan.targets[1].file,
        second.join("c_std/io.m2").canonicalize().unwrap()
    );
}

#[test]
fn universal_rejects_target_objects_but_accepts_sources() {
    let (root, paths, entry) = fixture();
    let file = root.path().join("objects/universal/c_std/io.sym");
    let mut manifest = artifacts::load(&file).unwrap().manifest;
    manifest.config.object = ObjectMode::Only;
    fs::write(&file, artifacts::to_binary(&manifest).unwrap()).unwrap();
    assert!(
        BuildPlan::discover_with_paths(&entry, &paths)
            .unwrap_err()
            .contains("Object = None")
    );
    fs::remove_file(file).unwrap();
    fs::create_dir_all(root.path().join("lib/universal/c_std")).unwrap();
    fs::write(
        root.path().join("lib/universal/c_std/io.m2"),
        "export func wrong() {}",
    )
    .unwrap();
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert!(plan.targets[1].file.ends_with("lib/universal/c_std/io.m2"));
}

#[test]
fn library_source_beats_local_binary_and_uses_separate_cache() {
    let (root, paths, entry) = fixture();
    fs::copy(
        root.path().join("objects/universal/c_std/io.sym"),
        root.path().join("local.sym"),
    )
    .unwrap();
    fs::create_dir_all(root.path().join("c_std")).unwrap();
    fs::copy(
        root.path().join("local.sym"),
        root.path().join("c_std/io.sym"),
    )
    .unwrap();
    let source = paths.imports.lib_dirs[0].join("c_std/io.m2");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "export func platform() {}").unwrap();
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert_eq!(plan.targets[1].file, source.canonicalize().unwrap());
    assert_eq!(
        plan.targets[1].artifact_stems[0],
        root.path()
            .join("objects/default")
            .join(HOST_TRIPLET)
            .join("c_std/io")
    );
}

#[test]
fn universal_source_precedes_target_binary_and_caches_for_target() {
    let (root, paths, entry) = fixture();
    let target = root
        .path()
        .join("objects")
        .join(HOST_TRIPLET)
        .join("c_std/io.sym");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(root.path().join("objects/universal/c_std/io.sym"), &target).unwrap();
    // With no source, the target-specific binary wins over universal declarations.
    let binary = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert_eq!(binary.targets[1].file, target.canonicalize().unwrap());

    let source = root.path().join("lib/universal/c_std/io.m2");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "export func platform() {}").unwrap();
    let plan = BuildPlan::discover_with_paths(&entry, &paths).unwrap();
    assert_eq!(plan.targets[1].file, source.canonicalize().unwrap());
    assert_eq!(plan.targets[1].module_name, "c_std::io");
    assert_eq!(
        plan.targets[1].artifact_stems[0],
        root.path()
            .join("objects/default")
            .join(HOST_TRIPLET)
            .join("c_std/io")
    );
}
