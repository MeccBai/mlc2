//! Serial, dependency-recursive startup. BuildPlan rejects cycles before generation.
use mlc_codegen::gens::instruction::IrValue;
use std::fmt::Write;

pub(super) fn append_initializer(
    ir: &mut String,
    name: &str,
    dependencies: &[String],
    own: &[String],
    dll_export: bool,
) {
    for dependency in dependencies {
        writeln!(ir, "declare void {}()", IrValue::Global(dependency.clone())).unwrap();
    }
    let guard = IrValue::Global(format!("{name}.done"));
    writeln!(ir, "{guard} = internal global i1 false").unwrap();
    let export = if dll_export { "dllexport " } else { "" };
    writeln!(ir, "define {export}void {}() {{\nentry:\n  %initialized = load i1, ptr {guard}\n  br i1 %initialized, label %done, label %initialize\ninitialize:", IrValue::Global(name.into())).unwrap();
    for init in dependencies.iter().chain(own) {
        writeln!(ir, "  call void {}()", IrValue::Global(init.clone())).unwrap();
    }
    writeln!(
        ir,
        "  store i1 true, ptr {guard}\n  br label %done\ndone:\n  ret void\n}}"
    )
    .unwrap();
}
