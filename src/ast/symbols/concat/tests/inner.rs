use super::*;

#[test]
fn inner_types_functions_and_interfaces_remain_in_their_defining_file() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, exports) = unit(
        &global,
        &mut package,
        "a",
        "unit Private {}; enum State { Ready, }; func<T> helper(x:T) -> T { return x; } Private::func<T> method(x:T) -> T { return x; } export unit Public {};",
    );
    let (b, _) = unit(&global, &mut package, "b", "unit Other {};");
    for name in ["a::Private", "a::State", "a::helper", "a::Private::method"] {
        let symbol = exports.inner[name];
        assert_eq!(package.lookup(name, a.config.file_id()), Some(symbol));
        assert_eq!(exports.lookup(name, a.config.file_id()), Some(symbol));
        assert!(package.lookup(name, b.config.file_id()).is_none());
        assert!(exports.lookup(name, b.config.file_id()).is_none());
        assert!(package.lookup(name, FileId::new(999)).is_none());
        assert!(!package.searchable.contains_key(name));
        assert_eq!(symbol.file_id(), a.config.file_id());
    }
    assert_eq!(
        package.lookup("a::Public", b.config.file_id()),
        exports.searchable.get("a::Public").copied()
    );
}

#[test]
fn identical_inner_short_names_are_isolated_after_multiple_publications() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, ea) = unit(
        &global,
        &mut package,
        "a",
        "func<T> helper(x:T) -> T { return x; }",
    );
    let (b, eb) = unit(
        &global,
        &mut package,
        "b",
        "func<T> helper(x:T) -> T { return x; }",
    );
    package.concat(ea.clone()).unwrap();
    package.concat(eb.clone()).unwrap();
    let left = ea.inner["a::helper"];
    let right = eb.inner["b::helper"];
    assert_ne!(left, right);
    assert_eq!(package.lookup("a::helper", a.config.file_id()), Some(left));
    assert_eq!(package.lookup("b::helper", b.config.file_id()), Some(right));
    assert!(package.lookup("a::helper", b.config.file_id()).is_none());
    assert!(package.lookup("b::helper", a.config.file_id()).is_none());
    for (symbol, id) in [(left, a.config.file_id()), (right, b.config.file_id())] {
        let ExportSymbol::Function {
            index,
            generic: true,
        } = symbol
        else {
            panic!("generic function expected")
        };
        assert!(
            package
                .file(id)
                .unwrap()
                .generics
                .function_templates
                .contains_key(&index)
        );
        assert_eq!(package.get_function(index, true).name, "helper");
    }
}

#[test]
fn invalid_or_foreign_inner_indices_leave_all_published_tables_unchanged() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, exports) = unit(
        &global,
        &mut package,
        "a",
        "unit Private {}; export unit Public {};",
    );
    let (_, foreign) = unit(&global, &mut package, "b", "export unit Other {};");
    let before_public = package.searchable.clone();
    let before_tables = package.exports.clone();
    let before_configs = package.configs.clone();
    for symbol in [
        ExportSymbol::Type(TypeIndex::empty()),
        foreign.searchable["b::Other"],
    ] {
        let mut invalid = exports.clone();
        invalid.inner.insert("a::Broken".into(), symbol);
        assert_eq!(
            package.concat(invalid),
            Err(ConcatError::InvalidExport("a::Broken".into()))
        );
        assert_eq!(package.searchable, before_public);
        assert_eq!(package.exports, before_tables);
        assert_eq!(package.configs, before_configs);
        assert!(package.lookup("a::Broken", a.config.file_id()).is_none());
    }
}

#[test]
fn inner_cannot_replace_another_files_searchable_symbol() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, ea) = unit(&global, &mut package, "a", "export unit Public {};");
    let (b, mut eb) = unit(&global, &mut package, "b", "unit Private {};");
    let expected = ea.searchable["a::Public"];
    eb.inner.insert("a::Public".into(), eb.inner["b::Private"]);
    let before = package.exports.clone();
    assert_eq!(
        package.concat(eb),
        Err(ConcatError::DuplicateSymbol("a::Public".into()))
    );
    assert_eq!(
        package.lookup("a::Public", a.config.file_id()),
        Some(expected)
    );
    assert_eq!(
        package.lookup("a::Public", b.config.file_id()),
        Some(expected)
    );
    assert_eq!(package.exports, before);
}

#[test]
fn replacing_a_files_export_snapshot_does_not_remove_arenas_or_other_files() {
    let global = GlobalConfig::new();
    let mut package = PackageSymbolTable::new();
    let (a, ea) = unit(
        &global,
        &mut package,
        "a",
        "unit Private {}; export unit Public {};",
    );
    let (b, eb) = unit(
        &global,
        &mut package,
        "b",
        "unit Private {}; export unit Public {};",
    );
    let ExportSymbol::Type(index) = ea.inner["a::Private"] else {
        panic!("type expected")
    };
    package
        .concat(ExportTable::empty(a.config.file_id()))
        .unwrap();
    assert!(package.lookup("a::Private", a.config.file_id()).is_none());
    assert!(package.lookup("a::Public", b.config.file_id()).is_none());
    assert_eq!(
        package.lookup("b::Private", b.config.file_id()),
        eb.inner.get("b::Private").copied()
    );
    assert_eq!(
        package.lookup("b::Public", a.config.file_id()),
        eb.searchable.get("b::Public").copied()
    );
    assert!(
        matches!(package.get_type(index), crate::ast::types::CompileType::Unit(v) if v.name == "a::Private")
    );
    assert!(package.config(a.config.file_id()).is_some());
}
