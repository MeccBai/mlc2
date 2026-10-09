use super::*;
use crate::{
    plan::BuildPlan,
    schedule::{self, BuildOptions, CompileOutput},
};
use std::fs;
mod universal;

fn project(root: &Path) {
    fs::write(root.join("Project.toml"), "[Project]\nName='demo'\nVersion='1'\nOutputDir='build'\nTriplet='project-triplet'\nLibDirs=['project-lib']\nLibCacheDir='project-cache'\n[[Project.Targets]]\nName='app'\nEntry='main.m2'\nType='bin'\nTriplet='target-triplet'\nLibDirs=['target-lib']\nLibCacheDir='target-cache'").unwrap();
}

#[test]
fn target_overrides_project_and_global_paths() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let global = GlobalConfig {
        lib_dirs: Some(vec![root.path().join("global-lib")]),
        lib_cache_dir: Some(root.path().join("global-cache")),
    };
    let exe = root.path().join("install/mlc.exe");
    let target = PathResolver::new(&exe, root.path(), Some("app"), &global).unwrap();
    assert_eq!(target.triplet, "target-triplet");
    assert_eq!(
        target.imports.lib_dirs,
        vec![root.path().join("target-lib/target-triplet")]
    );
    assert_eq!(
        target.lib_cache,
        Some(root.path().join("target-cache/target-triplet"))
    );
    assert_eq!(
        target.output,
        root.path().join("build/objects/target-triplet")
    );
    let project = PathResolver::new(&exe, root.path(), None, &global).unwrap();
    assert_eq!(project.triplet, "project-triplet");
    assert_eq!(
        project.imports.lib_dirs,
        vec![root.path().join("project-lib/project-triplet")]
    );
    assert!(PathResolver::new(&exe, root.path(), Some("unknown"), &global).is_err());
}

#[test]
fn global_config_paths_are_relative_to_config_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    assert!(GlobalConfig::load(&path).unwrap().lib_dirs.is_none());
    fs::write(&path, "LibDirs=['lib']\nLibCacheDir='cache'").unwrap();
    let global = GlobalConfig::load(&path).unwrap();
    let paths =
        PathResolver::new(&root.path().join("mlc.exe"), root.path(), None, &global).unwrap();
    assert_eq!(paths.triplet, HOST_TRIPLET);
    assert_eq!(
        paths.imports.lib_dirs,
        vec![root.path().join("lib").join(HOST_TRIPLET)]
    );
    assert_eq!(
        paths.lib_cache,
        Some(root.path().join("cache").join(HOST_TRIPLET))
    );
    fs::write(path, "LibDirs=42").unwrap();
    assert!(GlobalConfig::load(&root.path().join("config.toml")).is_err());
}

#[test]
fn default_layout_and_triplet_validation() {
    let root = tempfile::tempdir().unwrap();
    let paths = PathResolver::new(
        &root.path().join("mlc.exe"),
        root.path(),
        None,
        &GlobalConfig::default(),
    )
    .unwrap();
    assert_eq!(
        paths.imports.lib_dirs,
        vec![root.path().join("lib").join(HOST_TRIPLET)]
    );
    assert_eq!(
        paths.lib_fallback,
        root.path().join("build/objects/lib").join(HOST_TRIPLET)
    );
    for value in ["", "..", "../../bad", "x\\bad", "C:bad"] {
        assert!(validate_triplet(value).is_err());
    }
}

#[test]
fn imports_and_cache_fallback_are_target_specific() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let lib = root.path().join("target-lib/target-triplet");
    fs::create_dir_all(&lib).unwrap();
    fs::write(lib.join("leaf.m2"), "export func leaf() {}").unwrap();
    fs::write(root.path().join("main.m2"), "import leaf; func main() {}").unwrap();
    // A file where a cache directory is expected deterministically simulates write failure.
    fs::write(root.path().join("target-cache"), "blocked").unwrap();
    let paths = PathResolver::new(
        &root.path().join("mlc.exe"),
        root.path(),
        Some("app"),
        &GlobalConfig::default(),
    )
    .unwrap();
    let plan = BuildPlan::discover_with_paths(&root.path().join("main.m2"), &paths).unwrap();
    let options = BuildOptions {
        output: paths.output.clone(),
        target: paths.triplet.clone(),
        compiler_id: "test".into(),
        compiler_options: vec![],
        module_fingerprints: Default::default(),
        workers: 2,
    };
    let report = schedule::build(plan.clone(), options.clone(), |_| {
        Ok(CompileOutput {
            ir: None,
            global_init: None,
        })
    })
    .unwrap();
    assert!(report.succeeded(), "{:?}", report.results);
    assert_eq!(report.warnings.len(), 1);
    assert!(paths.output.join("main.sym").is_file());
    assert!(paths.lib_fallback.join("leaf.sym").is_file());
    let cached = schedule::build(plan, options, |_| Err("must reuse cache".into())).unwrap();
    assert!(cached.succeeded());
}
