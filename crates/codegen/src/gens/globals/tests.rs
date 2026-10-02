use super::*;
use crate::ast::{AbstractSyntaxTree, config::Config};
use crate::diagnostic::{error::ErrorHandle, warning::WarningHandle};

fn parse(source: &str) -> (AbstractSyntaxTree, PackageSymbolTable) {
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let config = Config::new(
        FileId::new(0),
        vec![],
        "test".into(),
        "globals.vl".into(),
        ErrorHandle::new("globals.vl".into()),
        WarningHandle::new("globals.vl".into()),
    );
    let mut ast = AbstractSyntaxTree::new(config, module.unwrap());
    let mut package = PackageSymbolTable::new();
    ast.analysis(&mut package);
    (ast, package)
}

fn generate(source: &str) -> (String, Vec<String>) {
    let (ast, package) = parse(source);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
    let mut generator = IrGenerator::new(crate::manifest::X86_64_PC_WINDOWS_MSVC.into());
    generator.generate_globals(ast.config.file_id(), &package);
    generator.generate_globals(ast.config.file_id(), &package);
    for function in ast.body {
        generator.generate(&package, function);
    }
    let initializers = generator.global_initializers().to_vec();
    (generator.finish().unwrap(), initializers)
}

fn verify(ir: String, suffix: &str) {
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path =
        std::env::temp_dir().join(format!("mlc2-globals-{suffix}-{}.obj", std::process::id()));
    compiler.emit(ir, &path).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn declaration_indices_and_qualified_names_are_preserved() {
    let (ast, package) =
        parse("global var z = 1; global var a = z+2; global val last = a+3; func main() {}");
    assert!(!ast.config.is_poisoned());
    let symbols = package.file(ast.config.file_id()).unwrap();
    assert_eq!(symbols.globals["z"].0, 0);
    assert_eq!(symbols.globals["a"].0, 1);
    assert_eq!(symbols.globals["last"].0, 2);
    assert_eq!(
        symbols
            .ordered_globals()
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>(),
        vec!["test::z", "test::a", "test::last"]
    );
}

#[test]
fn storage_and_init_are_internal_deduplicated_and_in_source_order() {
    let (ir, initializers) = generate(
        "global var z = 1; global var a = z+2; global val last = a+3; func main() { z = a; }",
    );
    assert_eq!(initializers, vec![".mlc.init.0"]);
    assert_eq!(
        ir.matches("define internal void @\".mlc.init.0\"").count(),
        1
    );
    for name in ["z", "a", "last"] {
        assert_eq!(
            ir.matches(&format!("@\"test::{name}\" = internal global"))
                .count(),
            1
        );
    }
    let init = ir[ir.find("define internal void").unwrap()..]
        .split("\n}\n")
        .next()
        .unwrap();
    let z = init.find("ptr @\"test::z\"").unwrap();
    let a = init.find("ptr @\"test::a\"").unwrap();
    let last = init.find("ptr @\"test::last\"").unwrap();
    assert!(z < a && a < last);
    verify(ir, "order");
}

#[test]
fn function_initializer_calls_refs_and_aggregate_writes() {
    let (ir, _) = generate(
        "unit Point { pub x:i32; }; global var count = 1; global var next = read(); global var point = Point{2}; global var array = [1,2]; global val reference = @mut count; func read() -> i32 { return count+3; } func main() { count = next; point.x = count; array[0] = point.x; $reference = array[0]; }",
    );
    assert!(ir.contains("@\"test::reference\" = internal global ptr"));
    assert!(ir.contains("call i32"), "{ir}");
    verify(ir, "aggregate");
}

#[test]
fn local_and_explicit_global_with_same_source_name_use_distinct_storage() {
    let (ir, _) =
        generate("global var value = 1; func main() { var value = 2; test::value = value; }");
    assert!(ir.contains("ptr @\"test::value\""));
    verify(ir, "shadow");
}

#[test]
fn global_val_assignment_is_still_rejected_by_frontend() {
    let (ast, _) = parse("global val locked = 1; func main() { locked = 2; }");
    assert!(ast.config.is_poisoned());
}

#[test]
fn empty_file_needs_no_initializer() {
    let (ir, initializers) = generate("func main() {}");
    assert!(initializers.is_empty());
    assert!(!ir.contains(".mlc.init"));
}
