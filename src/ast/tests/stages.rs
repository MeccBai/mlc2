use crate::ast::config::FileId;
use crate::ast::symbols::PackageSymbolTable;
use crate::ast::{
    AbstractSyntaxTree, ExportSymbol,
    arena::FuncIndex,
    config::Config,
    types::{CompileType::Enum, base_type::DataType},
};
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use std::collections::{HashMap, HashSet};

fn ast(id: usize, name: &str, source: &str) -> AbstractSyntaxTree {
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    AbstractSyntaxTree::new(
        Config::new(
            FileId::new(id),
            Vec::new(),
            name.into(),
            name.into(),
            ErrorHandle::new(name.into()),
            WarningHandle::new(name.into()),
        ),
        module.unwrap(),
    )
}

#[test]
fn construction_is_lazy_and_requires_retains_imports() {
    let ast = ast(
        7,
        "unit",
        "import std::io; unit Bad { member:Missing; }; func main() {}",
    );
    assert!(ast.body.is_empty());
    assert!(!ast.config.is_poisoned());
    assert_eq!(ast.config.file_id(), FileId::new(7));
    assert_eq!(ast.requires()[0].path(), &["std", "io"]);
}

#[test]
fn export_keeps_private_generic_templates_but_does_not_build_bodies() {
    let mut ast = ast(
        7,
        "unit",
        "export enum State { Ready, }; export unit Public {}; unit Private {}; func<T> helper(x:T) -> T { return x; } export func entry() { missing(); }",
    );
    let mut package = PackageSymbolTable::new();
    let exports = ast.export(&mut package);
    assert!(!ast.config.is_poisoned());
    assert!(ast.body.is_empty());
    assert!(exports.searchable.contains_key("unit::State"));
    assert!(exports.searchable.contains_key("unit::Public"));
    assert!(exports.searchable.contains_key("unit::entry"));
    assert!(!exports.searchable.contains_key("unit::Private"));
    assert!(exports.inner.contains_key("unit::Private"));
    assert!(matches!(
        exports.inner["unit::helper"],
        ExportSymbol::Function { generic: true, .. }
    ));
    let file = package.file(FileId::new(7)).unwrap();
    assert_eq!(file.generics.function_templates.len(), 1);
    assert!(exports.lookup("unit::helper", FileId::new(8)).is_none());
    assert!(exports.lookup("unit::helper", FileId::new(7)).is_some());
    assert!(exports.lookup("unit::entry", FileId::new(8)).is_some());
    assert_eq!(ast.export(&mut package), exports);
    ast.analysis(&mut package);
    assert!(ast.config.is_poisoned());
}

#[test]
fn export_then_analysis_and_direct_analysis_are_equivalent_and_idempotent() {
    let source = "export unit Point { x:i32; }; func<T> identity(x:T) -> T { return x; } func main() { var x = identity<i32>(1); }";
    let mut first = ast(3, "unit", source);
    let mut second = ast(3, "unit", source);
    let mut a = PackageSymbolTable::new();
    let mut b = PackageSymbolTable::new();
    first.export(&mut a);
    first.analysis(&mut a);
    second.analysis(&mut b);
    assert!(!first.config.is_poisoned());
    assert!(!second.config.is_poisoned());
    assert_eq!(first.body, second.body);
    assert_eq!(a.file(FileId::new(3)), b.file(FileId::new(3)));
    let before = first.body.clone();
    first.analysis(&mut a);
    assert_eq!(first.body, before);
}

#[test]
fn package_routes_equal_local_positions_to_distinct_files() {
    let mut a = ast(1, "a", "export enum State { Ready, }; func main() {}");
    let mut b = ast(2, "b", "export enum State { Busy, }; func main() {}");
    let mut package = PackageSymbolTable::new();
    let ea = a.export(&mut package);
    let eb = b.export(&mut package);
    let ExportSymbol::Type(ia) = ea.searchable["a::State"] else {
        panic!("type expected")
    };
    let ExportSymbol::Type(ib) = eb.searchable["b::State"] else {
        panic!("type expected")
    };
    assert_ne!(ia, ib);
    assert_eq!(ia.file_id(), FileId::new(1));
    assert_eq!(ib.file_id(), FileId::new(2));
    assert_eq!(HashSet::from([ia, ib]).len(), 2);
    assert!(matches!(package.get_type(ia), Enum(v) if v.name == "a::State"));
    assert!(matches!(package.get_type(ib), Enum(v) if v.name == "b::State"));
    a.analysis(&mut package);
    b.analysis(&mut package);
    assert!(!a.config.is_poisoned());
    assert!(!b.config.is_poisoned());
    assert!(package.register(FileId::new(1)).is_err());
    assert!(matches!(package.get_type(ia), Enum(v) if v.name == "a::State"));
}

#[test]
fn duplicate_file_identity_cannot_replace_an_existing_unit() {
    let mut package = PackageSymbolTable::new();
    let mut first = ast(4, "a", "unit A {};");
    first.analysis(&mut package);
    let mut second = ast(4, "b", "unit B {};");
    assert!(second.export(&mut package).searchable.is_empty());
    assert!(second.config.is_poisoned());
    assert!(
        package
            .file(FileId::new(4))
            .unwrap()
            .types
            .get_by_name(&"a::A".into())
            .is_some()
    );
}

#[test]
fn recursion_registry_does_not_reuse_another_files_same_named_instance() {
    use crate::ast::function::{InstantiationActives, instantiate::InstanceIndex};
    let mut ast = ast(9, "unit", "func<T> identity(x:T) -> T { return x; }");
    let mut package = PackageSymbolTable::new();
    ast.export(&mut package);
    let symbols = package.file_mut(FileId::new(9)).unwrap();
    let template = symbols
        .generics
        .functions
        .get_by_name(&"identity".into())
        .unwrap();
    let generic = symbols.generics.functions.get(template).generic_map["T"];
    let concrete = symbols.get_base(DataType::Integer, 32, true);
    let actives = InstantiationActives::default();
    let foreign_key = (FileId::new(10), "identity<i32>".into());
    actives.borrow_mut().insert(
        foreign_key.clone(),
        InstanceIndex::Function(FuncIndex::empty()),
    );
    let index = template.instantiation(
        &mut ast.config,
        &HashMap::from([(generic, concrete)]),
        symbols,
        Some(&actives),
        (0..1).into(),
    );
    assert!(!index.is_empty());
    assert_eq!(index.file_id(), FileId::new(9));
    assert!(!ast.config.is_poisoned());
    assert_eq!(actives.borrow().len(), 1);
    assert!(actives.borrow().contains_key(&foreign_key));
}

#[test]
fn package_rejects_duplicate_full_names_across_distinct_file_ids() {
    let mut package = PackageSymbolTable::new();
    let mut first = ast(1, "same", "unit A {};");
    let mut second = ast(2, "same", "unit A {};");
    first.export(&mut package);
    assert!(!first.config.is_poisoned());
    second.export(&mut package);
    assert!(second.config.is_poisoned());
    assert_eq!(package.declared_file("same::A"), Some(FileId::new(1)));
    assert!(format!("{:?}", second.config.error_handle().errors).contains("DuplicateSymbol"));
    assert!(
        package
            .file(FileId::new(2))
            .unwrap()
            .types
            .get_by_name(&"same::A".into())
            .is_none()
    );
}
