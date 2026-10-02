use super::*;
use crate::{
    artifacts,
    plan::{ImportResolver, TargetInput},
};
use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Project {
    root: tempfile::TempDir,
}
impl Project {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }
    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }
    fn plan(&self) -> BuildPlan {
        BuildPlan::discover(
            &self.root.path().join("main.m2"),
            &ImportResolver {
                lib_dirs: vec![self.root.path().join("lib")],
            },
        )
        .unwrap()
    }
    fn options(&self) -> BuildOptions {
        BuildOptions {
            output: self.root.path().join("out"),
            target: "x86_64-pc-windows-msvc".into(),
            compiler_id: "test-compiler-1".into(),
            compiler_options: vec![],
            workers: 2,
        }
    }
    fn build(&self, calls: Arc<AtomicUsize>) -> BuildReport {
        build(self.plan(), self.options(), move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompileOutput { ir: None })
        })
        .unwrap()
    }
}
fn count() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(0))
}
fn built(report: &BuildReport) -> usize {
    report
        .results
        .iter()
        .filter(|r| matches!(r, NodeResult::Built(_)))
        .count()
}

#[test]
fn unchanged_build_reuses_all_artifacts() {
    let project = Project::new();
    project.write("main.m2", "import leaf; func main() {}");
    project.write("leaf.m2", "export func f() -> i32 { return 1; }");
    let calls = count();
    assert_eq!(built(&project.build(calls.clone())), 2);
    let cached = project.build(calls.clone());
    assert!(cached.succeeded());
    assert_eq!(built(&cached), 0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn ordinary_body_change_does_not_rebuild_parent() {
    let project = Project::new();
    project.write("main.m2", "import leaf; func main() {}");
    project.write("leaf.m2", "export func f() -> i32 { return 1; }");
    project.build(count());
    project.write("leaf.m2", "export func f() -> i32 { return 2; }");
    let report = project.build(count());
    assert!(report.succeeded());
    assert_eq!(built(&report), 1);
    assert!(matches!(
        report.results[report
            .results
            .iter()
            .position(|r| r.artifact().unwrap().manifest.config.module_name == "main")
            .unwrap()],
        NodeResult::Cached(_)
    ));
}

#[test]
fn signature_and_generic_body_changes_invalidate_direct_users() {
    for (before, after) in [
        (
            "export func f() -> i32 { return 1; }",
            "export func f() -> i64 { return 1; }",
        ),
        (
            "func<T> f(x:T) -> T { return x; }",
            "func<T> f(x:T) -> T { var y = x; return y; }",
        ),
    ] {
        let project = Project::new();
        project.write("main.m2", "import leaf; func main() {}");
        project.write("leaf.m2", before);
        project.build(count());
        project.write("leaf.m2", after);
        let report = project.build(count());
        assert!(report.succeeded(), "{report:?}");
        assert_eq!(built(&report), 2);
    }
}

#[test]
fn compiler_options_invalidate_cache() {
    let project = Project::new();
    project.write("main.m2", "func main() {}");
    project.build(count());
    let mut options = project.options();
    options.compiler_options.push("optimize".into());
    let report = build(project.plan(), options, |_| Ok(CompileOutput { ir: None })).unwrap();
    assert_eq!(built(&report), 1);
}

#[test]
fn dependency_fingerprints_propagate_through_transitive_declarations() {
    let project = Project::new();
    project.write("main.m2", "import middle; func main() {}");
    project.write("middle.m2", "import leaf; export func middle() {}");
    project.write("leaf.m2", "export func f() -> i32 { return 1; }");
    assert!(project.build(count()).succeeded());
    project.write("leaf.m2", "export func f() -> i64 { return 1; }");
    let report = project.build(count());
    assert!(report.succeeded());
    assert_eq!(built(&report), 3);
}

#[test]
fn result_failure_blocks_dependants_but_not_independent_tasks() {
    let project = Project::new();
    project.write("main.m2", "import bad; import good; func main() {}");
    project.write("bad.m2", "func bad() {}");
    project.write("good.m2", "func good() {}");
    let report = build(project.plan(), project.options(), |request| {
        if request.target.module_name == "bad" {
            Err("intentional semantic failure".into())
        } else {
            Ok(CompileOutput { ir: None })
        }
    })
    .unwrap();
    assert!(!report.succeeded());
    assert_eq!(
        report
            .results
            .iter()
            .filter(|r| matches!(r, NodeResult::Failed { .. }))
            .count(),
        1
    );
    assert_eq!(
        report
            .results
            .iter()
            .filter(|r| matches!(r, NodeResult::Blocked { .. }))
            .count(),
        1
    );
    assert_eq!(built(&report), 1);
}

#[test]
fn llvm_emits_object_cache_detects_corruption_and_link_key_tracks_body() {
    let project = Project::new();
    project.write("main.m2", "func main() -> i32 { return 1; }");
    let compile = |request: CompileRequest<'_>| {
        let TargetInput::Source { text, .. } = &request.target.input else {
            panic!("unexpected declaration")
        };
        let value = if text.contains("return 2") { 2 } else { 1 };
        Ok(CompileOutput {
            ir: Some(format!("define i32 @main() {{ ret i32 {value} }}")),
        })
    };
    let first = build(project.plan(), project.options(), compile).unwrap();
    assert!(first.succeeded(), "{first:?}");
    assert_eq!(first.objects.len(), 1);
    fs::write(&first.objects[0], "corrupt").unwrap();
    assert!(artifacts::load(&first.results[0].artifact().unwrap().path).is_err());
    let repaired = build(project.plan(), project.options(), compile).unwrap();
    assert_eq!(built(&repaired), 1);
    assert_eq!(first.link_key, repaired.link_key);
    project.write("main.m2", "func main() -> i32 { return 2; }");
    let changed = build(project.plan(), project.options(), compile).unwrap();
    assert!(changed.succeeded());
    assert_ne!(first.link_key, changed.link_key);
}

#[test]
fn serialized_templates_keep_private_body_and_prebuilt_needs_no_callback() {
    let project = Project::new();
    project.write("main.m2", "func<T> f(x:T) -> T { return x; }");
    let report = project.build(count());
    assert!(report.succeeded(), "{report:?}");
    let artifact = report.results[0].artifact().unwrap();
    assert!(artifact.templates.is_some());
    assert!(artifact.manifest.symbols.functions.is_empty());
    assert_eq!(artifact.manifest.symbols.inner.len(), 1);
    let plan = BuildPlan::discover(&artifact.path, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let report = build(plan, project.options(), |_| {
        Err("must not compile declarations".into())
    })
    .unwrap();
    assert!(report.succeeded(), "{report:?}");
    assert!(matches!(report.results[0], NodeResult::Declaration(_)));
    let generic = artifact.path.with_extension("mg");
    fs::write(generic, "tampered").unwrap();
    assert!(artifacts::load(&artifact.path).is_err());
}

#[test]
fn local_directory_wins_and_library_fallback_is_deduplicated() {
    let project = Project::new();
    project.write(
        "main.m2",
        "import leaf; import leaf; import std::io; func main() {}",
    );
    let local = project
        .write("leaf.m2", "func local() {}")
        .canonicalize()
        .unwrap();
    project.write("lib/leaf.m2", "func system() {}");
    project.write("lib/std/io.m2", "func io() {}");
    let plan = project.plan();
    assert_eq!(plan.targets.len(), 3);
    assert_eq!(plan.targets[plan.entry.0].requires.len(), 2);
    assert!(plan.targets.iter().any(|t| t.file == local));
    assert!(plan.targets.iter().any(|t| t.module_name == "std::io"));
}

#[test]
fn local_declaration_wins_over_system_source() {
    let project = Project::new();
    project.write("main.m2", "func main() {}");
    let artifact = project.build(count()).results[0]
        .artifact()
        .unwrap()
        .clone();
    fs::copy(&artifact.path, project.root.path().join("leaf.toml")).unwrap();
    project.write("lib/leaf.m2", "func system() {}");
    let resolver = ImportResolver {
        lib_dirs: vec![project.root.path().join("lib")],
    };
    let import = mlc_syntax::ImportModule::new(vec!["leaf".into()], false);
    assert_eq!(
        resolver
            .resolve(&project.root.path().join("main.m2"), &import)
            .unwrap(),
        project
            .root
            .path()
            .join("leaf.toml")
            .canonicalize()
            .unwrap()
    );
}

#[test]
fn cycles_and_malformed_import_paths_are_rejected_before_execution() {
    let project = Project::new();
    project.write("main.m2", "import leaf; func main() {}");
    project.write("leaf.m2", "import main; func leaf() {}");
    let resolver = ImportResolver { lib_dirs: vec![] };
    let error = BuildPlan::discover(&project.root.path().join("main.m2"), &resolver).unwrap_err();
    assert!(error.contains("Import cycle"), "{error}");
    for segment in ["..", "a/b", "a.b", "C:", ""] {
        let import = mlc_syntax::ImportModule::new(vec![segment.into()], false);
        assert!(
            resolver
                .resolve(&project.root.path().join("main.m2"), &import)
                .is_err()
        );
    }
}
