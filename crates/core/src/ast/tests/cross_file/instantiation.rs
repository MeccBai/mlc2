use super::*;

#[test]
fn generic_interface_requirement_can_use_a_callers_type_without_importing_its_module() {
    for (visibility, accepted) in [("pub", true), ("", false)] {
        let mut package = PackageSymbolTable::new();
        let mut b = ast(
            1,
            "b",
            "generic HasGet { func get(self) -> i32; }; export func<T:HasGet> read(x:T) -> i32 { return x.get(); }",
        );
        publish(&mut package, &mut b);
        let mut c = ast(
            2,
            "c",
            &format!(
                "export unit P {{ pub x:i32; }}; {visibility} P::func get(self) -> i32 {{ return self->x; }}"
            ),
        );
        publish(&mut package, &mut c);
        analyze(&mut package, &mut c);
        package.set_imports(FileId::new(3), [FileId::new(1), FileId::new(2)]);
        let mut a = ast(
            3,
            "a",
            "func main() { var p = c::P{1}; var x = b::read<c::P>(p); }",
        );
        a.analysis(&mut package);
        assert_eq!(
            !a.config.is_poisoned(),
            accepted,
            "{:?}",
            a.config.error_handle()
        );
        assert!(
            package
                .lookup_in("c::P", &package.resolve_scope(FileId::new(1)))
                .is_none()
        );
    }
}

#[test]
fn exported_generic_instantiation_uses_definition_inner_and_arenas_once() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "unit Private<T> { pub value:T; }; export unit Box<T> { pub value:T; }; func<T> helper(x:T) -> T { return x; } export func<T> id(x:T) -> T { var p = Private<T>{x}; return helper<T>(p.value); }",
    );
    publish(&mut package, &mut b);
    for id in [2, 3] {
        package.set_imports(FileId::new(id), [FileId::new(1)]);
        let mut a = ast(
            id,
            &format!("a{id}"),
            "import b; func main() { var p = b::Box<i32>{1}; var x = b::id<i32>(p.value); }",
        );
        analyze(&mut package, &mut a);
    }
    let b = package.file(FileId::new(1)).unwrap();
    assert_eq!(
        b.function_instances.len(),
        2,
        "id and its private helper are shared across callers"
    );
    assert!(b.types.get_by_name(&"b::Box<i32>".into()).is_some());
    assert!(b.types.get_by_name(&"b::Private<i32>".into()).is_some());
    assert!(
        package
            .file(FileId::new(2))
            .unwrap()
            .function_instances
            .is_empty()
    );
    for instance in b.function_instances.values() {
        assert_eq!(instance.symbol.file_id(), FileId::new(1));
        assert!(!instance.body.is_empty());
    }
}

#[test]
fn generic_body_uses_definition_imports_not_callers_imports() {
    let mut package = PackageSymbolTable::new();
    let mut c = ast(0, "c", "export func<T> id(x:T) -> T { return x; }");
    publish(&mut package, &mut c);
    package.set_imports(FileId::new(1), [FileId::new(0)]);
    let mut b = ast(
        1,
        "b",
        "import c; export func<T> id(x:T) -> T { return c::id<T>(x); }",
    );
    publish(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(2, "a", "import b; func main() { var x = b::id<i32>(1); }");
    analyze(&mut package, &mut a);
    assert_eq!(
        package
            .file(FileId::new(0))
            .unwrap()
            .function_instances
            .len(),
        1
    );
    assert_eq!(
        package
            .file(FileId::new(1))
            .unwrap()
            .function_instances
            .len(),
        1
    );
    assert!(
        package
            .lookup_in("c::id", &package.resolve_scope(FileId::new(2)))
            .is_none()
    );
}

#[test]
fn directly_instantiating_private_foreign_symbol_is_rejected() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(1, "b", "func<T> id(x:T) -> T { return x; }");
    publish(&mut package, &mut b);
    let template = package
        .file(FileId::new(1))
        .unwrap()
        .generics
        .functions
        .get_by_name(&"id".into())
        .unwrap();
    let mut a = ast(2, "a", "func main() {}");
    analyze(&mut package, &mut a);
    let mut view = ResolveContext::new(
        &mut package,
        ResolveScope::new(FileId::new(2), [FileId::new(1)]),
    );
    assert!(
        template
            .instantiation(
                &mut a.config,
                &Default::default(),
                &mut view,
                None,
                (0..1).into()
            )
            .is_empty()
    );
    assert!(a.config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::PrivateInstantiation),
        (0..1).into()
    )));
    assert_eq!(view.scope().current, FileId::new(2));
}

#[test]
fn imported_generic_interfaces_instantiate_their_body_in_owner_arena() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "export unit P {}; api P::func<T> id(x:T) -> T { var y:T = x; return y; }",
    );
    publish(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(2, "a", "func main() { var x = b::P::id<i32>(1); }");
    analyze(&mut package, &mut a);
    let instances = &package.file(FileId::new(1)).unwrap().interface_instances;
    assert_eq!(instances.len(), 1);
    let instance = instances.values().next().unwrap();
    assert_eq!(instance.symbol.file_id(), FileId::new(1));
    assert_eq!(instance.body.len(), 2);
}

#[test]
fn imported_constraints_are_checked_using_cross_arena_types() {
    for (argument, accepted) in [("i32>(1)", true), ("bool>(true)", false)] {
        let mut package = PackageSymbolTable::new();
        let mut b = ast(
            1,
            "b",
            "export generic Number { std::generic::is_integer; };",
        );
        publish(&mut package, &mut b);
        package.set_imports(FileId::new(2), [FileId::new(1)]);
        let mut a = ast(
            2,
            "a",
            &format!(
                "func<T:b::Number> id(x:T) -> T {{ return x; }} func main() {{ var x = id<{argument}; }}"
            ),
        );
        a.analysis(&mut package);
        assert_eq!(
            !a.config.is_poisoned(),
            accepted,
            "{:?}",
            a.config.error_handle()
        );
    }
}

#[test]
fn foreign_template_failure_restores_scope_and_reports_at_caller_span() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(1, "b", "export func<T> bad(x:T) -> T { return missing; }");
    publish(&mut package, &mut b);
    let mut a = ast(2, "a", "func main() {}");
    analyze(&mut package, &mut a);
    let template = package
        .file(FileId::new(1))
        .unwrap()
        .generics
        .functions
        .get_by_name(&"bad".into())
        .unwrap();
    let generic = package.get_function(template, true).generic_map["T"];
    let concrete = package
        .file(FileId::new(2))
        .unwrap()
        .types
        .get_by_name(&"i32".into())
        .unwrap();
    let mut view = ResolveContext::new(
        &mut package,
        ResolveScope::new(FileId::new(2), [FileId::new(1)]),
    );
    let span = (0..1).into();
    let bindings = std::collections::HashMap::from([(generic, concrete)]);
    assert!(
        template
            .instantiation(&mut a.config, &bindings, &mut view, None, span)
            .is_empty()
    );
    assert_eq!(view.scope().current, FileId::new(2));
    assert!(a.config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(crate::diagnostic::error::ResolveError::UnknownVariable),
        span
    )));
    assert!(
        !view.file_config(FileId::new(1)).unwrap().is_poisoned(),
        "one failed instance must not poison its definition"
    );
}

#[test]
fn failed_foreign_instance_is_not_reused_as_a_valid_cached_symbol() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(1, "b", "export func<T> bad(x:T) -> T { return missing; }");
    publish(&mut package, &mut b);
    for id in [2, 3] {
        package.set_imports(FileId::new(id), [FileId::new(1)]);
        let mut a = ast(
            id,
            &format!("a{id}"),
            "func main() { var x = b::bad<i32>(1); }",
        );
        a.analysis(&mut package);
        assert!(a.config.is_poisoned());
        let b = package.file(FileId::new(1)).unwrap();
        assert!(b.functions.get_by_name(&"bad<i32>".into()).is_none());
        assert!(b.function_instances.is_empty());
    }
}

#[test]
fn foreign_recursive_function_reuses_active_owner_instance() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "export func<T> recur(x:T) -> T { return recur<T>(x); }",
    );
    publish(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(2, "a", "func main() { var x = b::recur<i32>(1); }");
    analyze(&mut package, &mut a);
    let instances = &package.file(FileId::new(1)).unwrap().function_instances;
    assert_eq!(instances.len(), 1);
    let instance = instances.values().next().unwrap();
    let Statement::ReturnBlock(ret) = &instance.body[0] else {
        panic!("return expected")
    };
    let Some(Expression::FuncCallE(call)) = &ret.value else {
        panic!("call expected")
    };
    assert_eq!(call.func, crate::ast::EnumBool::False(instance.symbol));
}

#[test]
fn foreign_recursive_unit_and_symbolic_arguments_keep_template_identity() {
    let mut package = PackageSymbolTable::new();
    let mut b = ast(
        1,
        "b",
        "export unit Node<T> { pub value:T; pub next:$Node<T>; }; export unit Box<T> { pub value:T; };",
    );
    publish(&mut package, &mut b);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut a = ast(
        2,
        "a",
        "func node(x:$b::Node<i32>) -> $b::Node<i32> { return x; } func<T> same(x:b::Box<T>) -> b::Box<T> { return x; } func main() { var x = same<i32>(b::Box<i32>{1}); }",
    );
    analyze(&mut package, &mut a);
    let node = package
        .file(FileId::new(1))
        .unwrap()
        .types
        .get_by_name(&"b::Node<i32>".into())
        .unwrap();
    let CompileType::Unit(unit) = package.get_type(node) else {
        panic!("unit expected")
    };
    assert_eq!(unit.members.len(), 2);
    let CompileType::Ref(reference) = package.get_type(unit.members[1].member_type) else {
        panic!("reference expected")
    };
    assert_eq!(reference.base, node);
    assert!(node.size(&package) >= 12);
}
