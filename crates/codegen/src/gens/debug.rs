//! Line tables only. Generic instances share a template declaration and source positions.
use crate::ast::{
    Function,
    config::{Config, FileId},
    symbols::PackageSymbolTable,
};
use crate::parser::out::Span;
use std::{collections::HashMap, path::Path};

// LLVM filters native NeverStepInto line numbers. The COFF writer restores
// this reserved placeholder to 0xf00f00 in the emitted line subsection.
pub const NEVER_STEP_INTO: usize = 0xefffff;

#[derive(Debug, Clone, Copy, Default)]
pub struct CodegenConfig {
    pub debug_info: bool,
    pub optimized: bool,
}

#[derive(Default)]
pub(super) struct DebugInfoBuilder {
    pub(super) optimized: bool,
    nodes: Vec<String>,
    files: HashMap<FileId, (usize, usize)>,
    units: Vec<usize>,
    templates: HashMap<(FileId, usize, usize), usize>,
    locations: HashMap<(usize, usize, usize), usize>,
}
fn quoted(text: &str) -> String {
    let mut result = String::new();
    for byte in text.bytes() {
        if (32..=126).contains(&byte) && byte != b'"' && byte != b'\\' {
            result.push(byte as char);
        } else {
            result.push_str(&format!("\\{byte:02X}"));
        }
    }
    result
}
impl DebugInfoBuilder {
    fn node(&mut self, text: String) -> usize {
        let id = self.nodes.len();
        self.nodes.push(text);
        id
    }
    fn file(&mut self, config: &Config) -> (usize, usize) {
        if let Some(ids) = self.files.get(&config.file_id()) {
            return *ids;
        }
        let path = config
            .current_file()
            .strip_prefix(r"\\?\")
            .unwrap_or(config.current_file());
        let path = Path::new(path);
        let file = self.node(format!(
            "!DIFile(filename: \"{}\", directory: \"{}\")",
            quoted(&path.file_name().unwrap_or_default().to_string_lossy()),
            quoted(&path.parent().unwrap_or(Path::new("")).to_string_lossy())
        ));
        let unit = self.node(format!("distinct !DICompileUnit(language: DW_LANG_C, file: !{file}, producer: \"mlc\", isOptimized: {}, runtimeVersion: 0, emissionKind: LineTablesOnly)", self.optimized));
        self.units.push(unit);
        self.files.insert(config.file_id(), (file, unit));
        (file, unit)
    }
    fn position(config: &Config, span: Span) -> (usize, usize) {
        config
            .source_position(span)
            .map(|p| (p.line, p.column))
            .unwrap_or((1, 1))
    }
    fn location(&mut self, scope: usize, config: &Config, span: Span, step_into: bool) -> usize {
        let (line, column) = if step_into {
            Self::position(config, span)
        } else {
            (NEVER_STEP_INTO, 0)
        };
        let key = (scope, line, column);
        if let Some(id) = self.locations.get(&key) {
            return *id;
        }
        let id = self.node(format!(
            "!DILocation(line: {line}, column: {column}, scope: !{scope})"
        ));
        self.locations.insert(key, id);
        id
    }
    pub(super) fn function(
        &mut self,
        function: &Function,
        name: &str,
        ir: &str,
        locations: &[(usize, Span, bool)],
        package: &PackageSymbolTable,
    ) -> String {
        let (file_id, instance) = match function {
            Function::Func(body) => (
                body.symbol.file_id(),
                package
                    .file(body.symbol.file_id())
                    .unwrap()
                    .function_instances
                    .contains_key(&body.symbol),
            ),
            Function::Interface(body) => (
                body.symbol.file_id(),
                package
                    .file(body.symbol.file_id())
                    .unwrap()
                    .interface_instances
                    .contains_key(&body.symbol),
            ),
        };
        if name.contains("__mlc_macro_") {
            return ir.to_owned();
        }
        let Some(config) = package.config(file_id) else {
            return ir.to_owned();
        };
        let (file, unit) = self.file(config);
        let (line, _) = Self::position(config, function.span());
        let types = self.node("!{null}".into());
        let signature = self.node(format!("!DISubroutineType(types: !{types})"));
        if instance {
            let key = (file_id, function.span().start, function.span().end);
            if !self.templates.contains_key(&key) {
                let name = quoted(name.split('<').next().unwrap_or(name));
                let id = self.node(format!("!DISubprogram(name: \"{name}\", scope: !{file}, file: !{file}, line: {line}, type: !{signature}, spFlags: 0)"));
                self.templates.insert(key, id);
            }
        }
        let declaration = if instance {
            let id = self.templates[&(file_id, function.span().start, function.span().end)];
            format!(", declaration: !{id}")
        } else {
            String::new()
        };
        let name = quoted(name);
        let flags = if self.optimized {
            "DISPFlagDefinition | DISPFlagOptimized"
        } else {
            "DISPFlagDefinition"
        };
        let scope = self.node(format!("distinct !DISubprogram(name: \"{name}\", linkageName: \"{name}\", scope: !{file}, file: !{file}, line: {line}, type: !{signature}, scopeLine: {line}, spFlags: {flags}, unit: !{unit}{declaration})"));
        let positions: HashMap<_, _> = locations
            .iter()
            .map(|(line, span, step)| (*line, (*span, *step)))
            .collect();
        let mut result = String::new();
        for (index, text) in ir.lines().enumerate() {
            if text.starts_with("define ") {
                result.push_str(text.strip_suffix(" {").unwrap());
                result.push_str(&format!(" !dbg !{scope} {{\n"));
            } else if text.starts_with("  ") && !text.trim_start().starts_with(';') {
                let (span, step) = positions
                    .get(&index)
                    .copied()
                    .unwrap_or((function.span(), true));
                let location = self.location(scope, config, span, step);
                result.push_str(text);
                result.push_str(&format!(", !dbg !{location}\n"));
            } else {
                result.push_str(text);
                result.push('\n');
            }
        }
        result
    }
    pub(super) fn finish(mut self) -> String {
        if self.units.is_empty() {
            return String::new();
        }
        let codeview = self.node("!{i32 2, !\"CodeView\", i32 1}".into());
        let version = self.node("!{i32 2, !\"Debug Info Version\", i32 3}".into());
        let mut result = format!(
            "!llvm.module.flags = !{{!{codeview}, !{version}}}\n!llvm.dbg.cu = !{{{}}}\n",
            self.units
                .iter()
                .map(|id| format!("!{id}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if !self.templates.is_empty() {
            let mut templates: Vec<_> = self.templates.values().copied().collect();
            templates.sort();
            result.push_str(&format!(
                "!mlc.generic.templates = !{{{}}}\n",
                templates
                    .iter()
                    .map(|id| format!("!{id}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for (id, text) in self.nodes.iter().enumerate() {
            result.push_str(&format!("!{id} = {text}\n"));
        }
        result
    }
}

/// NativePDB discards special CodeView rows instead of treating them as a call-site
/// stepping policy. An opaque, non-inlined forwarding frame gives default source
/// stepping the same boundary while leaving direct calls to the target debuggable.
pub(super) fn step_over_calls(code: &mut [super::instruction::Instruction], owner: &str) -> String {
    use super::instruction::{Instruction, IrValue, LlvmType, TypedValue};
    use std::fmt::Write;
    let mut helpers = String::new();
    let mut step_into = true;
    for (index, instruction) in code.iter_mut().enumerate() {
        if let Instruction::Location {
            step_into: step, ..
        } = instruction
        {
            step_into = *step;
            continue;
        }
        if step_into {
            continue;
        }
        let (func, target, args, callee) = match instruction {
            Instruction::MappedCall { func, target, args } => {
                (func.clone(), *target, args.clone(), None)
            }
            Instruction::IndirectCall {
                func,
                target,
                args,
                callee,
            } => (func.clone(), *target, args.clone(), Some(callee.clone())),
            _ => continue,
        };
        let name = format!("{owner}::__mlc_step_over_{index}");
        let mut forwarded = args.clone();
        if let Some(callee) = &callee {
            forwarded.push(TypedValue {
                ty: LlvmType::Ptr,
                value: callee.clone(),
            });
        }
        let abi = func.forwarding_abi(name.clone(), &forwarded);
        write!(
            helpers,
            "define internal {} {}(",
            abi.result().ty,
            IrValue::Global(name)
        )
        .unwrap();
        for (i, param) in abi.parameters().enumerate() {
            if i != 0 {
                helpers.push_str(", ");
            }
            write!(helpers, "{param} %r{i}").unwrap();
        }
        helpers.push_str(") noinline optnone \"disable-tail-calls\"=\"true\" {\nentry:\n  ");
        let values: Vec<_> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| TypedValue {
                ty: arg.ty.clone(),
                value: IrValue::Reg(i),
            })
            .collect();
        let result = (func.result().ty != LlvmType::Void).then_some(forwarded.len());
        let destination = callee
            .as_ref()
            .map(|_| IrValue::Reg(args.len()))
            .unwrap_or_else(|| IrValue::Global(func.name().into()));
        func.write_call_to(&mut helpers, result, &values, &destination)
            .expect("validated forwarding call");
        if let Some(reg) = result {
            writeln!(helpers, "\n  ret {} %r{reg}\n}}", func.result().ty).unwrap();
        } else {
            helpers.push_str("\n  ret void\n}\n");
        }
        *instruction = Instruction::MappedCall {
            func: Box::new(abi),
            target,
            args: forwarded,
        };
        step_into = true;
    }
    helpers
}
