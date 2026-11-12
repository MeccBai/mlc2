//! Host-native constant execution through shared libraries and workers.
mod cache;
mod platform;
use super::*;
use mlc_codegen::gens::instruction::IrValue;
use mlc_core::{
    ast::{
        self, TypeIndex,
        expression::{ConstValue, Expression, FuncCall, InitialList},
        symbols::{EnumBool, Resolution},
        types::{CompileType, TypeLookup, base_type::DataType},
    },
    diagnostic::warning::Warning,
};
use mlc_syntax::parser::out::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Layout {
    Integer {
        size: usize,
        signed: bool,
        boolean: bool,
    },
    Float {
        size: usize,
    },
    Pointer {
        size: usize,
        align: usize,
    },
    String,
    Aggregate {
        size: usize,
        fields: Vec<(usize, Layout)>,
    },
}
impl Layout {
    fn of(ty: TypeIndex, symbols: &(impl TypeLookup + ?Sized)) -> Result<Self, String> {
        Ok(match symbols.get_type(ty).unqualified() {
            CompileType::Base(base) => match base.data_type() {
                DataType::Float => Self::Float { size: base.size() },
                _ => Self::Integer {
                    size: base.size(),
                    signed: base.signed(),
                    boolean: base.data_type() == DataType::Boolean,
                },
            },
            CompileType::Enum(_) => Self::Integer {
                size: ty.size(symbols),
                signed: false,
                boolean: false,
            },
            CompileType::Ref(_) | CompileType::Function(_) => Self::Pointer {
                size: symbols.target_layout().ptr_size,
                align: symbols.target_layout().ptr_align,
            },
            CompileType::Str => Self::String,
            CompileType::List(list) => Self::Aggregate {
                size: ty.size(symbols),
                fields: (0..list.length)
                    .map(|i| {
                        Ok((
                            i * list.element_type.size(symbols),
                            Self::of(list.element_type, symbols)?,
                        ))
                    })
                    .collect::<Result<_, String>>()?,
            },
            CompileType::Unit(unit) if unit.variant.is_none() => {
                let mut offset: usize = 0;
                let mut fields = Vec::new();
                for member in &unit.members {
                    offset = offset.next_multiple_of(member.member_type.align(symbols));
                    fields.push((offset, Self::of(member.member_type, symbols)?));
                    offset += member.member_type.size(symbols);
                }
                Self::Aggregate {
                    size: ty.size(symbols),
                    fields,
                }
            }
            _ => return Err("This constant result has no serializable concrete layout".into()),
        })
    }
    fn size(&self) -> usize {
        match self {
            Self::Integer { size, .. } | Self::Float { size } | Self::Aggregate { size, .. } => {
                *size
            }
            Self::Pointer { size, .. } => *size,
            Self::String => std::mem::size_of::<usize>() + 8,
        }
    }
    fn pointers(&self) -> bool {
        match self {
            Self::Pointer { .. } => true,
            Self::Aggregate { fields, .. } => fields.iter().any(|(_, l)| l.pointers()),
            _ => false,
        }
    }
}

pub(super) struct Executor {
    plan: BuildPlan,
    modules: HashMap<usize, TempModule>,
    target: String,
    layout: mlc_core::ast::config::TargetLayout,
    cache_root: PathBuf,
    max_time: std::time::Duration,
    temp_ast_hash: OnceCell<Result<String, String>>,
    compiler_hash: OnceCell<Result<String, String>>,
    running: Cell<bool>,
    cache: RefCell<HashMap<String, Value>>,
}
impl Executor {
    pub(super) fn new(
        plan: BuildPlan,
        modules: HashMap<usize, TempModule>,
        target: String,
        layout: mlc_core::ast::config::TargetLayout,
        cache_root: PathBuf,
        max_time: std::time::Duration,
    ) -> Self {
        Self {
            plan,
            modules,
            target,
            layout,
            cache_root,
            max_time,
            temp_ast_hash: OnceCell::new(),
            compiler_hash: OnceCell::new(),
            running: Cell::new(false),
            cache: RefCell::new(HashMap::new()),
        }
    }
    fn cache_entry(
        &self,
        owner: usize,
        source: &str,
        expected: &Layout,
        library: &str,
    ) -> Result<cache::Entry, String> {
        let temp_ast_hash = self
            .temp_ast_hash
            .get_or_init(|| {
                let modules: Vec<_> = self
                    .plan
                    .order
                    .iter()
                    .map(|id| {
                        (
                            &self.plan.targets[id.0].module_name,
                            native_module(&self.modules[&id.0]),
                        )
                    })
                    .collect();
                mlc_builder::artifacts::semantic_hash(&modules)
            })
            .as_ref()
            .map_err(Clone::clone)?
            .clone();
        let compiler_hash = self
            .compiler_hash
            .get_or_init(|| {
                std::env::current_exe()
                    .map_err(|e| e.to_string())
                    .and_then(|file| {
                        std::fs::read(file)
                            .map(|bytes| mlc_builder::artifacts::content_hash(&bytes))
                            .map_err(|e| e.to_string())
                    })
            })
            .as_ref()
            .map_err(Clone::clone)?
            .clone();
        let call_hash = mlc_builder::artifacts::semantic_hash(&(
            &self.plan.targets[owner].module_name,
            source,
        ))?;
        let fingerprint = cache::Fingerprint {
            version: 2,
            compiler_build: mlc_builder::manifest::BUILD_NUMBER,
            compiler_hash,
            temp_ast_hash,
            call_hash,
            layout_hash: mlc_builder::artifacts::semantic_hash(expected)?,
            host: mlc_builder::paths::HOST_TRIPLET.into(),
            target: self.target.clone(),
            ptr_size: self.layout.ptr_size,
            ptr_align: self.layout.ptr_align,
            optimization: "O3".into(),
            library_file: library.into(),
        };
        Ok(cache::Entry::new(&self.cache_root, fingerprint))
    }
    fn execute(&self, owner: usize, source: String, expected: &Layout) -> Result<Value, String> {
        let host = mlc_builder::paths::HOST_TRIPLET;
        let platform = platform::HostOs::current()?;
        let entry = self.cache_entry(owner, &source, expected, platform.library())?;
        if let Some(value) = entry.load() {
            return Ok(value);
        }
        let compiler =
            mlc_builder::llvm::IrCompiler::with_o3(host).map_err(|error| error.to_string())?;
        let host_layout = compiler.target_layout();
        let lexed = mlc_syntax::lexer::tokenize(&source).map_err(|e| e.message())?;
        let (wrapper, errors) = mlc_syntax::parser::parse(&lexed.tokens, source.len());
        if !errors.is_empty() {
            return Err(format!("Cannot construct constant entry: {errors:?}"));
        }
        let mut package = PackageSymbolTable::new();
        let mut asts = Vec::new();
        for id in &self.plan.order {
            let target = &self.plan.targets[id.0];
            let mut module = native_module(&self.modules[&id.0]);
            if id.0 == owner {
                module.extend(wrapper.clone().unwrap());
            }
            let file = target.file.to_string_lossy().into_owned();
            let mut config = Config::new(
                FileId::new(id.0),
                vec![],
                target.module_name.clone(),
                file.clone(),
                ErrorHandle::new(file.clone()),
                WarningHandle::new(file),
            );
            config.set_target_layout(host_layout);
            package.set_imports(
                FileId::new(id.0),
                target.requires.iter().map(|id| FileId::new(id.0)),
            );
            let mut ast = AbstractSyntaxTree::new(config, module);
            ast.export(&mut package);
            if ast.config.is_poisoned() {
                super::checked(&ast).map_err(|report| report.to_string())?;
            }
            asts.push(ast);
        }
        for ast in &mut asts {
            ast.analysis(&mut package);
            if ast.config.is_poisoned() {
                super::checked(ast).map_err(|report| report.to_string())?;
            }
        }
        let wrapper_index = package
            .file(FileId::new(owner))
            .unwrap()
            .functions
            .get_by_name(&"__mlc_ctfe_value".to_string())
            .ok_or("Missing constant wrapper")?;
        let host_type = package
            .get_function(wrapper_index, false)
            .ret_type
            .ok_or("Constant evaluation needs a return value")?;
        if &Layout::of(host_type, &package)? != expected {
            return Err(
                "Host and target constant result layouts differ; this result cannot be embedded"
                    .into(),
            );
        }
        // The generated bridge calls this function outside the analyzed AST.
        package
            .file_mut(FileId::new(owner))
            .unwrap()
            .functions
            .get_mut(wrapper_index)
            .exported = true;
        let asts = mlc_optimizer::Optimizer::new(mlc_optimizer::OptimizeConfig::enabled())
            .optimize_all(asts, &mut package);
        super::names::qualify(&mut package, &self.plan);
        let abi = package
            .get_function(wrapper_index, false)
            .llvm_func(&package)
            .map_err(|e| e.to_string())?;
        let mut generator = IrGenerator::with_config(
            host.into(),
            mlc_codegen::gens::CodegenConfig {
                debug_info: false,
                optimized: true,
            },
        );
        let mut seen = std::collections::HashSet::new();
        for ast in asts {
            for function in ast.body {
                let key = match &function {
                    Function::Func(f) => format!("f{:?}", f.symbol),
                    Function::Interface(f) => format!("i{:?}", f.symbol),
                };
                if seen.insert(key) {
                    generator.generate(&package, function);
                }
            }
        }
        for (_, file) in package.arenas().iter() {
            for function in file.function_instances.values() {
                if seen.insert(format!("f{:?}", function.symbol)) {
                    generator.generate(&package, Function::Func(function.clone()));
                }
            }
        }
        let mut ir = generator.finish().map_err(|e| format!("{e:?}"))?;
        let name = IrValue::Global(abi.name().into());
        ir.push_str(&format!(
            "\ndefine {}void @__mlc_ctfe_entry(ptr %out) {{\nentry:\n",
            platform.export_attribute()
        ));
        if !abi.result().hidden.is_empty() {
            ir.push_str(&format!("  call void {name}(ptr %out)\n"));
        } else {
            let ty = &abi.result().ty;
            ir.push_str(&format!(
                "  %value = call {ty} {name}()\n  store {ty} %value, ptr %out, align 1\n"
            ));
        }
        ir.push_str("  ret void\n}\n");
        let directory = entry.staging()?;
        let object = directory.path().join(platform.object());
        let library = directory.path().join(platform.library());
        compiler.emit_o3(ir, &object).map_err(|e| e.to_string())?;
        let current = std::env::current_exe().map_err(|e| e.to_string())?;
        let root = if current
            .parent()
            .unwrap()
            .file_name()
            .is_some_and(|s| s == "deps")
        {
            current.parent().unwrap().parent().unwrap()
        } else {
            current.parent().unwrap()
        };
        platform.link(root, &object, &library, host)?;
        let schema = directory.path().join("layout.json");
        let result = directory.path().join("result.json");
        std::fs::write(&schema, serde_json::to_vec(expected).unwrap())
            .map_err(|e| e.to_string())?;
        let worker = root.join(format!("mlc-build{}", std::env::consts::EXE_SUFFIX));
        let mut child = Command::new(&worker)
            .arg("__mlc_ctfe_worker")
            .arg(&library)
            .arg(&schema)
            .arg(&result)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot launch {}: {e}", worker.display()))?;
        let started = std::time::Instant::now();
        loop {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                break;
            }
            if started.elapsed() >= self.max_time {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "CTFE execution exceeded MaxTime ({:.3}s)",
                    self.max_time.as_secs_f64()
                ));
            }
            std::thread::sleep(
                std::time::Duration::from_millis(5)
                    .min(self.max_time.saturating_sub(started.elapsed())),
            );
        }
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Constant worker failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let value = serde_json::from_slice(&std::fs::read(&result).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        entry.publish(directory.path(), platform.object())?;
        Ok(value)
    }
}
impl mlc_core::constant_eval::Executor for Executor {
    fn evaluate(
        &self,
        call: &FuncCall,
        symbols: &dyn Resolution,
        config: &mut Config,
        span: Span,
    ) -> Result<Option<Expression>, String> {
        if self.running.replace(true) {
            return Ok(None);
        }
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _reset = Reset(&self.running);
        let EnumBool::False(index) = call.func else {
            return Ok(None);
        };
        let function = symbols.get_function_regular(index);
        let Some(result_type) = function.ret_type else {
            return Ok(None);
        };
        let layout = Layout::of(result_type, symbols)?;
        if layout.pointers() {
            config.submit_warning(Warning::ConstantPointer, span);
        }
        if self.target != mlc_builder::paths::HOST_TRIPLET {
            config.submit_warning(
                Warning::ConstantTarget {
                    host: mlc_builder::paths::HOST_TRIPLET.into(),
                    target: self.target.clone(),
                },
                span,
            );
        }
        let mut bindings = Bindings::default();
        let arguments = call
            .args
            .iter()
            .map(|a| source(a, symbols, &mut bindings))
            .collect::<Result<Vec<_>, _>>()?;
        let returns = type_source(result_type, symbols);
        let source = format!(
            "func __mlc_ctfe_value()->{returns} {{{}return {}({});}}",
            bindings.statements.join(""),
            syntax_name(&function.name),
            arguments.join(",")
        );
        let key = format!("{}:{source}", index.file_id().index());
        let cached = self.cache.borrow().get(&key).cloned();
        let value = if let Some(value) = cached {
            value
        } else {
            let value = self.execute(index.file_id().index(), source, &layout)?;
            self.cache.borrow_mut().insert(key, value.clone());
            value
        };
        Ok(Some(expression(result_type, &value, symbols)?))
    }
}
fn source(
    value: &Expression,
    symbols: &dyn Resolution,
    bindings: &mut Bindings,
) -> Result<String, String> {
    Ok(match value {
        Expression::ConstValueE(value, ..) => {
            if value.ty.is_empty() {
                "null".into()
            } else {
                match symbols.get_type(value.ty).unqualified() {
                    CompileType::Base(base) if base.data_type() == DataType::Boolean => {
                        value.value.clone()
                    }
                    CompileType::Base(base) => {
                        let mut literal = value.value.clone();
                        if base.data_type() == DataType::Float && !literal.contains(['.', 'e', 'E'])
                        {
                            literal.push_str(".0");
                        }
                        format!("cast<{}>({literal})", base.name())
                    }
                    CompileType::Enum(enumeration) => {
                        let i: usize = value.value.parse().map_err(|_| "Invalid constant enum")?;
                        format!(
                            "{}::{}",
                            enumeration.name,
                            enumeration.variants.get(i).ok_or("Invalid enum ordinal")?
                        )
                    }
                    _ => return Err(
                        "Pointer constant arguments must be constructed inside the const function"
                            .into(),
                    ),
                }
            }
        }
        Expression::VarValueE(variable, ..) if variable.constant => {
            source(&variable.init_val, symbols, bindings)?
        }
        Expression::InitListE(InitialList::String { value }, ..) => quote(value),
        Expression::InitListE(InitialList::Array { values, .. }, ..) => format!(
            "[{}]",
            values
                .iter()
                .map(|v| source(v, symbols, bindings))
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        ),
        Expression::InitListE(
            InitialList::List {
                onwer: Some(owner),
                values,
            },
            ..,
        ) => format!(
            "{}{{{}}}",
            type_source(*owner, symbols),
            values
                .iter()
                .map(|v| source(v, symbols, bindings))
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        ),
        Expression::FuncCallE(call, ..) if call.callee.is_none() => {
            let EnumBool::False(i) = call.func else {
                return Err("Cannot serialize an interface call".into());
            };
            let function = symbols.get_function_regular(i);
            let name = match function.builtin() {
                Some(ast::builtins::Builtin::Cast) => {
                    format!("cast<{}>", type_source(function.ret_type.unwrap(), symbols))
                }
                Some(ast::builtins::Builtin::ConstCStr) => "const_c_str".into(),
                Some(ast::builtins::Builtin::StrLen) => "str::len".into(),
                _ => function.name.clone(),
            };
            format!(
                "{name}({})",
                call.args
                    .iter()
                    .map(|v| source(v, symbols, bindings))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",")
            )
        }
        Expression::CompositeE(value, ..) => {
            let mut values = value
                .members
                .iter()
                .map(|v| source(&v.clone().to_expression(), symbols, bindings));
            let mut text = values.next().ok_or("Empty constant expression")??;
            for (operator, value) in value.operators.iter().zip(values) {
                text.push_str(binary_operator(operator)?);
                text.push_str(&value?);
            }
            format!("({text})")
        }
        Expression::UnaryExprE(
            ast::expression::UnaryExpr::Access(ast::expression::Access::Member {
                base,
                indirect,
                name,
            }),
            ..,
        ) => {
            format!(
                "({}){}{name}",
                source(&base.clone().to_expression(), symbols, bindings)?,
                if *indirect { "->" } else { "." }
            )
        }
        Expression::UnaryExprE(
            ast::expression::UnaryExpr::Access(ast::expression::Access::Index { base, index }),
            ..,
        ) => {
            let base = source(base, symbols, bindings)?;
            format!("({base})[{}]", source(index, symbols, bindings)?)
        }
        Expression::UnaryExprE(ast::expression::UnaryExpr::Operator { op, value }, ..) => {
            use mlc_syntax::operators::Operator;
            let operator = match op {
                Operator::AddressOf => "@",
                Operator::MutOf => "@mut ",
                Operator::Dereference => "$",
                Operator::Negate => "-",
                Operator::LogicalNot => "!",
                Operator::BitNot => "~",
                _ => return Err("Unsupported constant argument operator".into()),
            };
            if matches!(op, Operator::AddressOf | Operator::MutOf) {
                format!(
                    "{operator}{}",
                    source_place(&value.clone().to_expression(), symbols, bindings)?
                )
            } else {
                format!(
                    "{operator}({})",
                    source(&value.clone().to_expression(), symbols, bindings)?
                )
            }
        }
        _ => return Err("The compile-time argument did not reduce to a serializable value".into()),
    })
}
fn expression(
    ty: TypeIndex,
    value: &Value,
    symbols: &dyn Resolution,
) -> Result<Expression, String> {
    let span = (0..0).into();
    Ok(match symbols.get_type(ty).unqualified() {
        CompileType::Str => Expression::InitListE(
            InitialList::String {
                value: value.as_str().ok_or("Invalid string result")?.into(),
            },
            span,
        ),
        CompileType::List(list) => Expression::InitListE(
            InitialList::Array {
                ty,
                values: value
                    .as_array()
                    .ok_or("Invalid array result")?
                    .iter()
                    .map(|v| expression(list.element_type, v, symbols))
                    .collect::<Result<_, _>>()?,
            },
            span,
        ),
        CompileType::Unit(unit) => Expression::InitListE(
            InitialList::List {
                onwer: Some(ty),
                values: unit
                    .members
                    .iter()
                    .zip(value.as_array().ok_or("Invalid unit result")?)
                    .map(|(m, v)| expression(m.member_type, v, symbols))
                    .collect::<Result<_, _>>()?,
            },
            span,
        ),
        _ => Expression::ConstValueE(
            ConstValue {
                ty,
                value: value.as_str().ok_or("Invalid scalar result")?.into(),
            },
            span,
        ),
    })
}

pub(crate) fn worker() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(2).collect();
    if args.len() != 3 {
        return Err("Invalid constant worker arguments".into());
    }
    let layout: Layout =
        serde_json::from_slice(&std::fs::read(&args[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let library = platform::Library::open(Path::new(&args[0]))?;
    let mut output = vec![0u128; layout.size().div_ceil(16).max(1)];
    let value = unsafe {
        let function = library.entry()?;
        function(output.as_mut_ptr().cast());
        read(&layout, output.as_ptr().cast())
    };
    // Copy library-backed strings before unloading the library.
    let encoded = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    std::fs::write(&args[2], encoded).map_err(|e| e.to_string())?;
    Ok(())
}

unsafe fn read(layout: &Layout, data: *const u8) -> Value {
    unsafe {
        match layout {
            Layout::Integer {
                size,
                signed,
                boolean,
            } => {
                let mut raw = [0u8; 16];
                #[cfg(target_endian = "little")]
                let n = {
                    std::ptr::copy_nonoverlapping(data, raw.as_mut_ptr(), *size);
                    u128::from_le_bytes(raw)
                };
                #[cfg(target_endian = "big")]
                let n = {
                    std::ptr::copy_nonoverlapping(data, raw.as_mut_ptr().add(16 - size), *size);
                    u128::from_be_bytes(raw)
                };
                let text = if *boolean {
                    (n & 1 != 0).to_string()
                } else if *signed {
                    let shift = 128 - size * 8;
                    ((n << shift) as i128 >> shift).to_string()
                } else {
                    n.to_string()
                };
                Value::String(text)
            }
            Layout::Float { size } => Value::String(
                if *size == 4 {
                    std::ptr::read_unaligned(data.cast::<f32>()) as f64
                } else {
                    std::ptr::read_unaligned(data.cast::<f64>())
                }
                .to_string(),
            ),
            Layout::Pointer { .. } => {
                Value::String(std::ptr::read_unaligned(data.cast::<usize>()).to_string())
            }
            Layout::String => {
                let pointer = std::ptr::read_unaligned(data.cast::<*const u8>());
                let length =
                    std::ptr::read_unaligned(data.add(std::mem::size_of::<usize>()).cast::<u64>())
                        as usize;
                Value::String(if length == 0 {
                    String::new()
                } else {
                    String::from_utf8_lossy(std::slice::from_raw_parts(pointer, length))
                        .into_owned()
                })
            }
            Layout::Aggregate { fields, .. } => Value::Array(
                fields
                    .iter()
                    .map(|(offset, field)| read(field, data.add(*offset)))
                    .collect(),
            ),
        }
    }
}

fn quote(value: &str) -> String {
    let mut text = String::from("\"");
    for c in value.chars() {
        match c {
            '\0' => text.push_str("\\0"),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            '\\' => text.push_str("\\\\"),
            '"' => text.push_str("\\\""),
            _ => text.push(c),
        }
    }
    text.push('"');
    text
}

fn type_source(ty: TypeIndex, symbols: &dyn Resolution) -> String {
    syntax_name(&symbols.get_type(ty).unqualified().format(symbols))
}
fn syntax_name(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<_> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let mut j = i + 1;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        out.push(if *c == ',' && j > i + 1 && chars.get(j) == Some(&']') {
            ':'
        } else {
            *c
        });
    }
    out
}

#[derive(Default)]
struct Bindings {
    statements: Vec<String>,
    places: HashMap<usize, String>,
}
fn source_place(
    value: &Expression,
    symbols: &dyn Resolution,
    bindings: &mut Bindings,
) -> Result<String, String> {
    use ast::expression::{Access, UnaryExpr};
    match value {
        Expression::UnaryExprE(
            UnaryExpr::Access(Access::Member {
                base,
                indirect,
                name,
            }),
            ..,
        ) => {
            let base = source_place(&base.clone().to_expression(), symbols, bindings)?;
            return Ok(format!(
                "{base}{}{name}",
                if *indirect { "->" } else { "." }
            ));
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), ..) => {
            let base = source_place(base, symbols, bindings)?;
            return Ok(format!("{base}[{}]", source(index, symbols, bindings)?));
        }
        Expression::UnaryExprE(
            UnaryExpr::Operator {
                op: mlc_syntax::operators::Operator::Dereference,
                value,
            },
            ..,
        ) => {
            return Ok(format!(
                "$({})",
                source(&value.clone().to_expression(), symbols, bindings)?
            ));
        }
        _ => (),
    }
    let identity = match value {
        Expression::VarValueE(variable, ..) => Some(std::rc::Rc::as_ptr(variable) as usize),
        _ => None,
    };
    if let Some(name) = identity.and_then(|key| bindings.places.get(&key)) {
        return Ok(name.clone());
    }
    let value = source(value, symbols, bindings)?;
    let name = format!("__mlc_ctfe_arg{}", bindings.statements.len());
    bindings.statements.push(format!("var {name}={value};"));
    if let Some(key) = identity {
        bindings.places.insert(key, name.clone());
    }
    Ok(name)
}

fn binary_operator(operator: &mlc_syntax::operators::Operator) -> Result<&'static str, String> {
    use mlc_syntax::operators::Operator::*;
    Ok(match operator {
        Add => " + ",
        Subtract => " - ",
        Multiply => " * ",
        Divide => " / ",
        Remainder => " % ",
        ShiftLeft => " << ",
        ShiftRight => " >> ",
        Equal => " == ",
        NotEqual => " != ",
        Less => " < ",
        LessOrEqual => " <= ",
        Greater => " > ",
        GreaterOrEqual => " >= ",
        BitAnd => " & ",
        BitOr => " | ",
        BitXor => " ^ ",
        LogicalAnd => " && ",
        LogicalOr => " || ",
        _ => return Err("Unsupported constant binary operator".into()),
    })
}

fn native_module(module: &TempModule) -> TempModule {
    let mut module = module.clone();
    module.retain_mut(|(item, _)| {
        match item {
            TempGlobalStmt::Variable(v) if v.value_type != mlc_syntax::ValueType::Constant => {
                return false;
            }
            TempGlobalStmt::Func(f) if !f.symbol.constant => f.body = None,
            TempGlobalStmt::Interface(f) => f.body = None,
            _ => (),
        }
        true
    });
    module
}
