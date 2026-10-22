use super::*;
use mlc_builder::plan::ImportResolver;

#[test]
fn optimizer_switches_control_ir_without_changing_required_calls() {
    let root = tempfile::tempdir().unwrap();
    let entry = root.path().join("main.m2");
    std::fs::write(&entry, "global var hits=0+0;func touch()->i32{hits=hits+1;return hits;}func unused(){}func main()->i32{var discarded=touch();var arithmetic=1+2;if(true){hits=hits+1;}else{hits=hits+10;}return hits-2;}").unwrap();
    let plan = BuildPlan::discover(&entry, &ImportResolver { lib_dirs: vec![] }).unwrap();
    let compiler = mlc_builder::llvm::IrCompiler::init("x86_64-pc-windows-msvc").unwrap();
    for (label, options) in [
        ("off", mlc_optimizer::OptimizeConfig::disabled()),
        ("on", mlc_optimizer::OptimizeConfig::enabled()),
    ] {
        let generated =
            generate_with_options(&plan, "x86_64-pc-windows-msvc", TargetKind::Bin, options)
                .unwrap();
        let mut objects = vec![];
        for (id, ir) in generated
            .modules
            .into_iter()
            .chain(generated.supplement.into_iter().map(|ir| (usize::MAX, ir)))
        {
            if id != usize::MAX {
                assert!(ir.contains("call i32 @\"main::touch\""), "{ir}");
                assert_eq!(
                    ir.contains("define void @\"main::unused\""),
                    label == "off",
                    "{ir}"
                );
                assert_eq!(ir.contains("if.then"), label == "off", "{ir}");
            }
            let object = root.path().join(format!("{label}-{id}.obj"));
            compiler.emit(ir, &object).unwrap();
            objects.push(object);
        }
        #[cfg(windows)]
        {
            let executable = root.path().join(format!("{label}.exe"));
            let linked = super::test_support::link(&objects, &executable);
            assert!(
                linked.status.success(),
                "{}",
                String::from_utf8_lossy(&linked.stderr)
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
}

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
        let output = super::test_support::link(&objects, &executable);
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
