use super::*;
use mlc_builder::plan::ImportResolver;

#[test]
fn pruned_control_flow_preserves_side_effects_and_unmatched_match_fallthrough() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(
        &entry,
        r#"
        global var hits = 0;
        func touch() -> bool { hits = hits+1; return true; }
        func main() -> i32 {
            if (1 < 2) { hits = hits+1; } else { hits = hits+100; }
            if (false && touch()) { hits = hits+100; }
            while (false) { touch(); }
            for i in [3,1] { touch(); }
            for j in [0,3] {
                match (99) { 1 => { break; }, _ => {} }
                hits = hits+1;
            }
            match (2) { _ => { hits = hits+100; }, 2 => { hits = hits+1; } }
            if (touch() || true) { hits = hits+1; }
            return hits-7;
        }
    "#,
    )
    .unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let generated = generate(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin).unwrap();
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    let mut objects = Vec::new();
    let mut modules = generated.modules;
    if let Some(ir) = generated.supplement {
        modules.push((usize::MAX, ir));
    }
    for (id, ir) in modules {
        let object = root.path().join(format!("module{id}.obj"));
        compiler.emit(ir, &object).unwrap();
        objects.push(object);
    }
    #[cfg(windows)]
    {
        let executable = root.path().join("main.exe");
        let clang = std::env::var_os("MLC_TEST_CLANG").unwrap_or_else(|| "clang".into());
        let output = std::process::Command::new(clang)
            .arg("-fuse-ld=lld")
            .args(objects)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::process::Command::new(executable)
                .status()
                .unwrap()
                .code(),
            Some(0)
        );
    }
}
