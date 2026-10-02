use super::*;
use crate::ast::{AbstractSyntaxTree, arena::TypeIndex, config::GlobalConfig};
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;

mod inner;

fn unit(
    global: &GlobalConfig,
    package: &mut PackageSymbolTable,
    prefix: &str,
    source: &str,
) -> (AbstractSyntaxTree, ExportTable) {
    let lexed = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&lexed.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let config = global.config(
        Vec::new(),
        prefix.into(),
        prefix.into(),
        ErrorHandle::new(prefix.into()),
        WarningHandle::new(prefix.into()),
    );
    let mut ast = AbstractSyntaxTree::new(config, module.unwrap());
    let exports = ast.export(package);
    assert!(
        !ast.config.is_poisoned(),
        "{:?}",
        ast.config.error_handle().errors
    );
    (ast, exports)
}

#[test]
fn entry_package_collects_multiple_units_without_moving_arenas() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, _) = unit(
        &global,
        &mut package,
        "a",
        "export unit Public {}; func main() {}",
    );
    let (mut b, exports) = unit(
        &global.clone(),
        &mut package,
        "b",
        "export unit Data {}; func<T> helper(x:T) -> T { return x; } func main() { var x = helper<i32>(1); }",
    );
    let exported = exports.searchable["b::Data"];
    package.concat(exports).unwrap();
    assert!(package.file(a.config.file_id()).is_some());
    assert!(package.file(b.config.file_id()).is_some());
    assert_eq!(
        package.lookup("b::Data", a.config.file_id()),
        Some(exported)
    );
    assert!(package.lookup("b::helper", a.config.file_id()).is_none());
    assert!(package.lookup("b::helper", b.config.file_id()).is_some());
    assert_eq!(
        package.config(b.config.file_id()).unwrap().symbol_prefix(),
        "b"
    );
    let ExportSymbol::Type(index) = exported else {
        panic!("type expected")
    };
    assert_eq!(index.file_id(), b.config.file_id());
    assert!(
        matches!(package.get_type(index), crate::ast::types::CompileType::Unit(v) if v.name == "b::Data")
    );
    b.analysis(&mut package);
    assert!(!b.config.is_poisoned());
    assert!(!b.body.is_empty());
}

#[test]
fn concat_requires_an_arena_registered_in_the_entry_package() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let config = global.config(
        Vec::new(),
        "missing".into(),
        "missing".into(),
        ErrorHandle::new("missing".into()),
        WarningHandle::new("missing".into()),
    );
    assert_eq!(
        package.concat(ExportTable::empty(config.file_id())),
        Err(ConcatError::UnknownFile(config.file_id()))
    );
    assert!(package.config(config.file_id()).is_none());
}

#[test]
fn duplicate_names_abort_the_whole_export_batch() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, _) = unit(&global, &mut package, "a", "export unit Data {};");
    let (b, mut exports) = unit(&global, &mut package, "b", "export unit Data {};");
    let symbol = exports.searchable["b::Data"];
    exports.searchable.insert("a::Data".into(), symbol);
    exports.searchable.insert("b::Extra".into(), symbol);
    assert_eq!(
        package.concat(exports),
        Err(ConcatError::DuplicateSymbol("a::Data".into()))
    );
    assert!(package.lookup("b::Extra", a.config.file_id()).is_none());
    assert!(package.lookup("a::Data", a.config.file_id()).is_some());
    assert!(package.lookup("b::Data", a.config.file_id()).is_some());
}

#[test]
fn export_publication_is_repeatable_but_rejects_invalid_indices_atomically() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (ast, exports) = unit(&global, &mut package, "a", "export unit Data {};");
    package.concat(exports.clone()).unwrap();
    let expected = package.lookup("a::Data", ast.config.file_id());
    let mut invalid = exports;
    invalid
        .searchable
        .insert("a::Broken".into(), ExportSymbol::Type(TypeIndex::empty()));
    assert_eq!(
        package.concat(invalid),
        Err(ConcatError::InvalidExport("a::Broken".into()))
    );
    assert_eq!(package.lookup("a::Data", ast.config.file_id()), expected);
    assert!(package.lookup("a::Broken", ast.config.file_id()).is_none());
}
