use mlc_syntax::manifest::SOURCE_SUFFIX;
mod constant_control;
mod numeric_initialization;
mod resources;
mod function_pointer;
mod union;

use super::*;
use crate::ast::{
    AbstractSyntaxTree,
    config::{Config, FileId},
};
use crate::diagnostic::{error::ErrorHandle, warning::WarningHandle};

fn parse(source: &str) -> (AbstractSyntaxTree, PackageSymbolTable) {
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let config = Config::new(
        FileId::new(0),
        vec![],
        String::new(),
        format!("generation{}", SOURCE_SUFFIX),
        ErrorHandle::new(format!("generation{}", SOURCE_SUFFIX)),
        WarningHandle::new(format!("generation{}", SOURCE_SUFFIX)),
    );
    let mut ast = AbstractSyntaxTree::new(config, module.unwrap());
    let mut package = PackageSymbolTable::new();
    ast.analysis(&mut package);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
    (ast, package)
}

fn emit(source: &str, suffix: &str) -> String {
    let (ast, package) = parse(source);
    let mut generator = IrGenerator::new(crate::manifest::X86_64_PC_WINDOWS_MSVC.into());
    for function in &ast.body {
        generator.add_func_decl(function, &package);
        generator.generate(&package, function.clone());
        generator.generate(&package, function.clone());
    }
    let ir = generator.finish().unwrap();
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path =
        std::env::temp_dir().join(format!("mlc2-function-{suffix}-{}.obj", std::process::id()));
    compiler.emit(ir.clone(), &path).unwrap();
    std::fs::remove_file(path).unwrap();
    ir
}

#[test]
fn parameters_assignments_and_one_return_block() {
    let ir = emit(
        "func sum(a:i32,b:i32) -> i32 { var c = a+b; c = c+1; return c; var dead = 3; }",
        "basic",
    );
    assert_eq!(ir.matches("define i32").count(), 1);
    assert!(!ir.contains("declare i32"));
    assert_eq!(ir.matches("ret i32").count(), 1);
}

#[test]
fn branches_preserve_distinct_exit_snapshots() {
    let (ast, package) = parse(
        "func choose(a:bool) -> i32 { var outer = 1; if (a) { var yes = 2; return yes; } else { var no = 3; return no; } }",
    );
    let generated = FunctionGenerator::generate(&ast.body[0], &package);
    let returns: Vec<_> = generated
        .exits
        .iter()
        .filter(|exit| exit.kind == ExitKind::Return)
        .collect();
    assert_eq!(returns.len(), 2);
    assert_ne!(returns[0].from, returns[1].from);
    let names: Vec<_> = returns
        .iter()
        .map(|exit| {
            generated
                .variables
                .between(exit.from, exit.to)
                .iter()
                .map(|v| v.name.clone())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(names[0].iter().any(|name| name == "yes"));
    assert!(!names[0].iter().any(|name| name == "no"));
    assert!(names[1].iter().any(|name| name == "no"));
    emit(
        "func choose(a:bool) -> i32 { if (a) { return 2; } else { return 3; } }",
        "branches",
    );
}

#[test]
fn loops_match_and_nested_exits() {
    emit(
        "func main() { var n = 0; for i in [0,4] { var local = i; if (i == 1) { continue; } match (i) { 2 => { break; }, _ => { n = n+1; } } } while (n < 5) { n = n+1; if (n == 4) { continue; } break; } return; }",
        "loops",
    );
}

#[test]
fn calls_and_member_assignments() {
    emit(
        "unit Point { pub x:i32; }; func add(a:i32) -> i32 { return a+1; } func main() { var point = Point{1}; point.x = add(point.x); var array = [1,2]; array[0] = point.x; var ref = @mut point.x; $ref = 4; }",
        "calls",
    );
}

#[test]
fn break_cleans_loop_binding_but_continue_keeps_it() {
    let (ast, package) = parse(
        "func main() { var outer = 0; for i in [0,2] { var local = i; if (i == 0) { continue; } else { break; } } }",
    );
    let generated = FunctionGenerator::generate(&ast.body[0], &package);
    for kind in [ExitKind::Break, ExitKind::Continue] {
        let exit = generated
            .exits
            .iter()
            .find(|exit| exit.kind == kind)
            .unwrap();
        let names: Vec<_> = generated
            .variables
            .between(exit.from, exit.to)
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        assert!(names.contains(&"local"));
        assert!(!names.contains(&"outer"));
        assert_eq!(names.contains(&"i"), kind == ExitKind::Break);
    }
    let entry_end = generated.ir.find("br label").unwrap();
    assert!(!generated.ir[entry_end..].contains("alloca"));
}

#[test]
fn indirect_returns_and_parameter_storage() {
    emit(
        "unit Pair { pub a:i64; pub b:i64; }; func identity(value:Pair) -> Pair { return value; } func consume(a:i64,b:i64) { var pair = Pair{a,b}; var next = identity(pair); next.a = a; } func main() {}",
        "indirect",
    );
}

#[test]
fn recursive_aggregate_and_context_typed_initializers() {
    let ir = emit(
        "unit Inner { pub x:i8; pub y:i32; }; unit Outer { pub inner:Inner; pub n:i32; }; func main() { var p:Inner = {1,2}; var q = Outer{{3,4},5}; var array = [q,q]; array[0].n = p.y; }",
        "nested-init",
    );
    assert!(ir.contains("llvm.memset.p0.i64"));
    assert!(ir.contains("store i8 1"));
    assert!(!ir.contains("trunc i32"));
}

#[test]
fn strings_copy_decoded_bytes_and_deduplicate_module_constants() {
    let ir = emit(
        r#"global var text = "中\n"; func first() { var local = "中\n"; var empty = ""; } func second() { var again = "中\n"; }"#,
        "strings",
    );
    assert_eq!(
        ir.matches("private unnamed_addr constant [4 x i8]").count(),
        1
    );
    assert!(ir.contains("c\"\\E4\\B8\\AD\\0A\""));
    assert_eq!(
        ir.matches("declare void @\"llvm.memcpy.p0.p0.i64\"")
            .count(),
        1
    );
    assert!(ir.contains("[0 x i8]"));
}

#[test]
fn split_parameter_definition_and_return_roundtrip_validate_in_llvm() {
    let (ast, package) = parse(
        "unit Pair { pub a:i64; pub b:i64; }; func identity(pair:Pair) -> Pair { pair.a = pair.b; return pair; }",
    );
    let function = &ast.body[0];
    let abi = match function {
        Function::Func(f) => package
            .get_function(f.symbol, false)
            .llvm_func(&package)
            .unwrap(),
        _ => unreachable!(),
    };
    let abi = abi
        .with_split_parameter(0, vec![LlvmType::Int(64), LlvmType::Int(64)], &package)
        .unwrap()
        .with_split_return(vec![LlvmType::Int(64), LlvmType::Int(64)], &package)
        .unwrap();
    let generated = FunctionGenerator::generate_with_abi(function, &package, abi.clone());
    assert!(generated.ir.contains("(i64 %r0, i64 %r1)"));
    assert!(generated.ir.contains("insertvalue"));
    let logical = IrGenerator::type_lowering(abi.params()[0].source, &package);
    let argument = LlvmValue {
        ty: logical.clone(),
        reg: Some(0),
        in_reg: false,
        code: vec![
            Instruction::Alloca {
                target: 0,
                ty: logical.clone(),
                count: None,
                align: Some(8),
            },
            Instruction::Store {
                pointer: IrValue::Reg(0),
                value: TypedValue {
                    ty: logical.clone(),
                    value: IrValue::Val("{ i64 7, i64 9 }".into()),
                },
                align: Some(8),
            },
        ],
    };
    let (next, mut result) = abi.call_values(1, &[argument], &package);
    assert_eq!(result.ty, logical);
    assert!(!result.in_reg);
    result.code.push(Instruction::Load {
        target: next,
        ty: logical.clone(),
        pointer: IrValue::Reg(result.reg.unwrap()),
        align: None,
    });
    result.code.push(Instruction::Return {
        value: Some(TypedValue {
            ty: logical.clone(),
            value: IrValue::Reg(next),
        }),
    });
    let mut generator = IrGenerator::new(crate::manifest::X86_64_PC_WINDOWS_MSVC.into());
    generator.ensure_type(
        abi.params()[0].source,
        &package,
        &mut std::collections::HashSet::new(),
    );
    let mut ir = generator.finish().unwrap();
    ir.push_str(&generated.ir);
    ir.push_str(&format!("define {logical} @caller() {{\nentry:\n"));
    InstructionBuffer {
        instructions: result.code,
    }
    .write_to(&mut ir)
    .unwrap();
    ir.push_str("}\n");
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path = std::env::temp_dir().join(format!("mlc2-split-{}.obj", std::process::id()));
    compiler.emit(ir, &path).unwrap();
    std::fs::remove_file(path).unwrap();
}
