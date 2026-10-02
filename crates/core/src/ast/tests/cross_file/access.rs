use super::*;

#[test]
fn publishing_derived_types_does_not_expose_private_foreign_names_in_inner() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "unit Secret { pub x:i32; }; export func make() -> Secret { return Secret{1}; }",
    );
    publish(&mut package, &mut b);
    analyze(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(
        2,
        "a",
        "func main() { val first = b::make(); var second = first; }",
    );
    analyze(&mut package, &mut a);
    publish(&mut package, &mut a);
    let mut context = package.resolve_context(FileId::new(2));
    let temp = TempType::Path(TempPath {
        segments: vec!["b".into(), "Secret".into()],
    });
    assert!(resolve_type(&mut a.config, (temp, (0..1).into()), &mut context, None).is_none());
    assert!(a.config.is_poisoned());
}

#[test]
fn imported_types_functions_references_arrays_and_enum_values_build_real_ast() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "export unit P { pub x:i32; }; export enum E { One, Two }; export func sum(x:i32) -> i32 { return x+1; }",
    );
    publish(&mut package, &mut b);
    analyze(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(
        2,
        "a",
        "import b; using Alias = b::P; func main() -> i32 { var p:Alias = b::P{1}; var r:$b::P = @p; var ps = [p,p]; var e:b::E = b::E::One; var n = b::sum(r->x); return n; }",
    );
    analyze(&mut package, &mut a);
    let local = package.file(FileId::new(2)).unwrap();
    let ty = package.lookup("b::P", FileId::new(2)).unwrap();
    let ExportSymbol::Type(index) = ty else {
        panic!("type expected")
    };
    assert_eq!(index.file_id(), FileId::new(1));
    assert!(
        local.types.get_by_name(&"b::P".into()).is_none(),
        "no imported declaration copies"
    );
    assert_eq!(index.size(package.arenas()), 4);
}

#[test]
fn loaded_but_unimported_and_private_names_are_not_searchable() {
    for source in [
        "func main() { var p = b::P{1}; }",
        "func main() { var p = b::Private{1}; }",
        "func main() { var x = b::private(); }",
    ] {
        let mut package = PackageSymbolTable::new();
        let mut b = ast(
            1,
            "b",
            "export unit P { pub x:i32; }; unit Private { x:i32; }; func private() -> i32 { return 1; }",
        );
        publish(&mut package, &mut b);
        if !source.contains("b::P{") {
            package.set_imports(FileId::new(2), [FileId::new(1)]);
        }
        let mut a = ast(2, "a", source);
        a.analysis(&mut package);
        assert!(a.config.is_poisoned(), "unexpected acceptance: {source}");
        assert_eq!(a.config.error_handle().errors.len(), 1);
    }
}

#[test]
fn foreign_reference_resolution_preserves_original_base_index() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(1, "b", "export unit P { pub x:i32; };");
    publish(&mut package, &mut b);
    let mut a = ast(2, "a", "func main() {}");
    analyze(&mut package, &mut a);
    let mut view = ResolveContext::new(
        &mut package,
        ResolveScope::new(FileId::new(2), [FileId::new(1)]),
    );
    let span = (0..1).into();
    let ty = TempType::Reference {
        inner: Box::new((
            TempType::Path(TempPath {
                segments: vec!["b".into(), "P".into()],
            }),
            span,
        )),
        mutable: false,
    };
    let index = resolve_type(&mut a.config, (ty, span), &mut view, None).unwrap();
    let CompileType::Ref(reference) = view.get_type(index) else {
        panic!("reference expected")
    };
    assert_eq!(index.file_id(), FileId::new(2));
    assert_eq!(reference.base.file_id(), FileId::new(1));
    assert_eq!(index.format(&view), "$b::P");
}

#[test]
fn imported_public_interfaces_work_but_private_members_stay_inaccessible() {
    for (source, accepted) in [
        (
            "func main() -> i32 { var p = b::P{1}; return p.get(); }",
            true,
        ),
        ("func main() -> i32 { var p = b::P{1}; return p.x; }", false),
        (
            "func main() -> i32 { var p = b::P{1}; return p.private(); }",
            false,
        ),
    ] {
        let mut package = PackageSymbolTable::new();
        let mut b = ast(
            1,
            "b",
            "export unit P { x:i32; }; pub P::func get(self) -> i32 { return self->x; } P::func private(self) -> i32 { return self->x; }",
        );
        publish(&mut package, &mut b);
        analyze(&mut package, &mut b);
        package.set_imports(FileId::new(2), [FileId::new(1)]);
        let mut a = ast(2, "a", source);
        a.analysis(&mut package);
        assert_eq!(
            !a.config.is_poisoned(),
            accepted,
            "{source}: {:?}",
            a.config.error_handle()
        );
    }
}

#[test]
fn value_qualification_does_not_make_private_foreign_type_name_visible() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "unit Secret { pub x:i32; }; export func make() -> Secret { return Secret{1}; }",
    );
    publish(&mut package, &mut b);
    analyze(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(
        2,
        "a",
        "func main() { val first = b::make(); var second = first; var hidden:b::Secret = second; }",
    );
    a.analysis(&mut package);
    assert!(a.config.is_poisoned());
    assert_eq!(a.config.error_handle().errors.len(), 1);
}
