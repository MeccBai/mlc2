use super::*;

#[test]
fn parses_import() {
    assert_eq!(parse_ok("import std::io;").len(), 1);
}

#[test]
fn variadic_marker_is_the_last_function_or_interface_parameter() {
    let module = parse_ok(
        "func tt(...) -> int; func mixed(a:i32, ...) -> int; Point::func method(self, a:i32, ...); generic G { func required(self, ...); };",
    );

    let TempGlobalStmt::Func(first) = &module[0].0 else {
        panic!("expected a function");
    };
    assert_eq!(first.symbol.params[0].name, "...");
    assert!(first.symbol.params[0].ty.is_none());

    let TempGlobalStmt::Func(mixed) = &module[1].0 else {
        panic!("expected a function");
    };
    assert_eq!(mixed.symbol.params.len(), 2);
    assert_eq!(mixed.symbol.params[1].name, "...");
    assert!(mixed.symbol.params[1].ty.is_none());

    let TempGlobalStmt::Interface(method) = &module[2].0 else {
        panic!("expected an interface");
    };
    assert!(method.symbol.has_self);
    assert_eq!(method.symbol.params[1].name, "...");
    assert!(method.symbol.params[1].ty.is_none());

    let TempGlobalStmt::Generic(generic) = &module[3].0 else {
        panic!("expected a generic requirement");
    };
    let out::TempConstraints::Interface(required) = &generic.requirements[0].0 else {
        panic!("expected an interface requirement");
    };
    assert_eq!(required.params[0].name, "...");
    assert!(required.params[0].ty.is_none());
}

#[test]
fn rejects_parameters_after_variadic_marker() {
    for source in [
        "func tt(..., a:i32);",
        "func tt(a:i32, ..., b:i32);",
        "Point::func tt(..., a:i32);",
        "generic G { func tt(..., a:i32); };",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "{source} should be rejected");
    }
}

#[test]
fn only_interfaces_accept_pub_and_api_visibility() {
    for declaration in [
        "import std::io;",
        "unit Point {};",
        "using PointAlias = Point;",
        "generic Number {};",
        "enum Color { Red };",
        "func run();",
    ] {
        for visibility in ["pub", "api"] {
            let source = format!("{visibility} {declaration}");
            let lexed = tokenize(&source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "{source} should be rejected");
            assert_eq!(errors[0].span().into_range(), 0..visibility.len());
        }
    }

    parse_ok("pub Point::func inspect(self);");
    parse_ok("api Point::func inspect(self);");
    parse_ok("export func run();");
}

#[test]
fn parser_keeps_using_only_as_a_temp_statement() {
    let module = parse_ok("using bytes = Buffer<u8>; ");
    let TempGlobalStmt::Using(using) = &module[0].0 else {
        panic!("expected a temporary using declaration");
    };

    assert_eq!(using.name, "bytes");
    assert!(matches!(using.target.0, TempType::Generic { .. }));
}

#[test]
fn parses_global_variables_without_export_visibility() {
    let module = parse_ok(
        "global var counter:i32 = 0; global const limit:i32 = 10; global val fixed:i32 = 3;",
    );

    assert!(matches!(
        &module[0].0,
        TempGlobalStmt::Variable(variable)
            if variable.name == "counter" && variable.value_type == crate::language::ValueType::Flex
    ));
    assert!(matches!(
        &module[1].0,
        TempGlobalStmt::Variable(variable)
            if variable.name == "limit" && variable.value_type == crate::language::ValueType::Constant
    ));
    assert!(matches!(
        &module[2].0,
        TempGlobalStmt::Variable(variable)
            if variable.name == "fixed" && variable.value_type == crate::language::ValueType::Final
    ));
}

#[test]
fn variable_declarations_require_initializers() {
    for source in [
        "func main() { var x:i32; }",
        "func main() { const x:i32; }",
        "func main() { val x:i32; }",
        "global var x:i32;",
        "global const x:i32;",
        "global val x:i32;",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "{source} should be rejected");
    }

    parse_ok("func main() { var x:i32 = 1; val z = 3; const y = 2; }");
    parse_ok("global var x:i32 = 1; global val z = 3; global const y = 2;");
}

#[test]
fn parses_public_and_private_unit_members() {
    let module = parse_ok("unit Point { pub x:i32; y:i32; };");
    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!("expected a unit");
    };

    assert!(unit.members[0].public);
    assert!(!unit.members[1].public);
}

#[test]
fn rejects_export_as_unit_member_visibility() {
    let source = "unit Point { export x:i32; };";
    let lexed = tokenize(source).unwrap();
    let (_, errors) = parse(&lexed.tokens, source.len());
    assert!(!errors.is_empty());
}

#[test]
fn rejects_exported_global_variables() {
    for source in [
        "export global var counter:i32 = 0;",
        "api global var counter:i32 = 0;",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "accepted invalid source: {source}");
    }
}

#[test]
fn parses_explicit_generics_method_mutability_enum_and_c_abi() {
    let module = parse_ok(
        "generic number { std::generic::is_integer; }; \
         #[c_abi]# func<T:number> identity(value:T) -> T { return value; } \
         unit Point { x:i32; }; Point::func set(mut self, value:i32) {} \
         enum State { Waiting, Running };",
    );

    let TempGlobalStmt::Func(function) = &module[1].0 else {
        panic!("expected a free function");
    };
    assert_eq!(function.symbol.attributes, ["c_abi"]);
    assert_eq!(function.symbol.generics[0].name, "T");
    assert_eq!(
        function.symbol.generics[0]
            .constraint
            .as_ref()
            .unwrap()
            .0
            .segments,
        ["number"]
    );

    let TempGlobalStmt::Interface(method) = &module[3].0 else {
        panic!("expected a method");
    };
    assert_eq!(method.symbol.owner.as_ref().unwrap().0.segments, ["Point"]);
    assert!(method.symbol.has_self);
    assert!(method.symbol.mutable);
    assert_eq!(method.symbol.params.len(), 1);
    assert_eq!(method.symbol.params[0].name, "value");

    assert!(matches!(
        &module[4].0,
        TempGlobalStmt::Enum(TempEnum { name, variants, .. })
            if name == "State"
                && variants.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>()
                    == ["Waiting", "Running"]
    ));
}

#[test]
fn generic_interface_requirement_has_only_a_symbol() {
    let module = parse_ok(
        "generic Addable { pub func add(mut self, rhs:i32) -> i32; Point::func make() -> Point; };",
    );
    let TempGlobalStmt::Generic(generic) = &module[0].0 else {
        panic!("expected a generic declaration");
    };
    let out::TempConstraints::Interface(symbol) = &generic.requirements[0].0 else {
        panic!("expected an interface requirement");
    };
    assert!(symbol.owner.is_none());
    assert!(symbol.has_self);
    assert!(symbol.mutable);
    assert_eq!(symbol.visibility, out::TempVisibility::Public);
    assert_eq!(symbol.name, "add");
    assert_eq!(symbol.params[0].name, "rhs");

    let out::TempConstraints::Interface(owned) = &generic.requirements[1].0 else {
        panic!("expected an owned interface requirement");
    };
    assert_eq!(owned.owner.as_ref().unwrap().0.segments, ["Point"]);
    assert!(!owned.has_self);
    assert!(!owned.mutable);
    assert!(owned.params.is_empty());
}

#[test]
fn preserves_generic_requirement_and_constraint_spans() {
    let source = "generic G { std::generic::max_bits<16>; func check(self); }; unit U<T:G> {};";
    let module = parse_ok(source);
    let TempGlobalStmt::Generic(generic) = &module[0].0 else {
        panic!("expected a generic declaration");
    };
    assert_eq!(
        &source[generic.requirements[0].1.into_range()],
        "std::generic::max_bits<16>;"
    );
    assert_eq!(
        &source[generic.requirements[1].1.into_range()],
        "func check(self);"
    );

    let TempGlobalStmt::Unit(unit) = &module[1].0 else {
        panic!("expected a unit declaration");
    };
    let (_, constraint_span) = unit.generics[0].constraint.as_ref().unwrap();
    assert_eq!(&source[constraint_span.into_range()], "G");
    assert!(!generic.dump().contains("Position:"));
    assert_eq!(generic.dump_with_span().matches("Position:").count(), 2);
}

#[test]
fn preserves_declaration_and_nested_name_spans() {
    let source = "unit Point<T> { x:i32; }; func run(value:i32,...) {} \
                  Point::func get(self, index:i32) {} \
                  generic Numeric { func check(self); }; \
                  enum State { Ready }; using Alias = i32; global var count:i32 = 0;";
    let module = parse_ok(source);
    let at = |span: out::Span| &source[span.into_range()];

    let TempGlobalStmt::Unit(unit) = &module[0].0 else {
        panic!("expected unit");
    };
    assert_eq!(at(unit.name_span), "Point");
    assert_eq!(at(unit.generics[0].name_span), "T");
    assert_eq!(at(unit.members[0].name_span), "x");

    let TempGlobalStmt::Func(function) = &module[1].0 else {
        panic!("expected function");
    };
    assert_eq!(at(function.symbol.name_span), "run");
    assert_eq!(at(function.symbol.params[0].name_span), "value");
    assert_eq!(at(function.symbol.params[1].name_span), "...");

    let TempGlobalStmt::Interface(interface) = &module[2].0 else {
        panic!("expected interface");
    };
    assert_eq!(at(interface.symbol.name_span), "get");
    assert_eq!(at(interface.symbol.params[0].name_span), "index");

    let TempGlobalStmt::Generic(generic) = &module[3].0 else {
        panic!("expected generic");
    };
    assert_eq!(at(generic.name_span), "Numeric");
    let out::TempConstraints::Interface(requirement) = &generic.requirements[0].0 else {
        panic!("expected interface requirement");
    };
    assert_eq!(at(requirement.name_span), "check");

    let TempGlobalStmt::Enum(enumeration) = &module[4].0 else {
        panic!("expected enum");
    };
    assert_eq!(at(enumeration.name_span), "State");
    assert_eq!(at(enumeration.variants[0].1), "Ready");

    let TempGlobalStmt::Using(using) = &module[5].0 else {
        panic!("expected using");
    };
    assert_eq!(at(using.name_span), "Alias");
    let TempGlobalStmt::Variable(variable) = &module[6].0 else {
        panic!("expected global variable");
    };
    assert_eq!(at(variable.name_span), "count");
}

#[test]
fn distinguishes_free_functions_and_interfaces_without_receivers() {
    let module = parse_ok("func make() {} Point::func make() {}");
    assert!(matches!(&module[0].0, TempGlobalStmt::Func(function)
        if function.symbol.name == "make" && function.body.is_some()));
    assert!(matches!(&module[1].0, TempGlobalStmt::Interface(interface)
        if interface.symbol.owner.as_ref().unwrap().0.segments == ["Point"]
            && !interface.symbol.has_self && !interface.symbol.mutable && interface.body.is_some()));
}

#[test]
fn interface_owner_keeps_its_own_span() {
    let source = "pkg::Point::func read(self);";
    let module = parse_ok(source);
    let TempGlobalStmt::Interface(interface) = &module[0].0 else {
        panic!("expected an interface");
    };
    let (owner, span) = interface.symbol.owner.as_ref().unwrap();
    assert_eq!(owner.segments, ["pkg", "Point"]);
    assert_eq!(&source[span.into_range()], "pkg::Point");
}

#[test]
fn receiver_flags_live_only_on_interface_symbol() {
    let module = parse_ok(
        "Point::func read(self, value:i32) {} \
         Point::func write(mut self, value:i32) {} \
         Point::func make(value:i32) {}",
    );
    for ((item, _), (has_self, mutable)) in
        module
            .iter()
            .zip([(true, false), (true, true), (false, false)])
    {
        let TempGlobalStmt::Interface(interface) = item else {
            panic!("expected an interface");
        };
        assert_eq!(interface.symbol.has_self, has_self);
        assert_eq!(interface.symbol.mutable, mutable);
        assert_eq!(interface.symbol.params.len(), 1);
        assert_eq!(interface.symbol.params[0].name, "value");
    }
}

#[test]
fn temp_dump_includes_interface_and_generic_requirements() {
    let module = parse_ok("generic G { func run(self); }; Point::func run(self) {}");
    let generic_dump = module[0].0.dump();
    assert!(generic_dump.contains("Interface Requirement: run"));
    assert!(generic_dump.contains("Has self: true"));

    let interface_dump = module[1].0.dump_with_span(module[1].1);
    assert!(interface_dump.contains("Interface: run"));
    assert!(interface_dump.contains("Owner: Some(\"Point\")"));
    assert!(interface_dump.contains("Position:"));
}

#[test]
fn temp_dump_covers_every_global_item() {
    let module = parse_ok(
        "import std::io; unit Point { x:i32; }; func run() {} \
         Point::func get(self) {} using Number = i32; \
         generic Numeric { std::generic::is_integer; }; \
         enum State { Ready }; global var count:i32 = 0;",
    );
    let labels = [
        "Import:",
        "Unit:",
        "Function:",
        "Interface:",
        "Using:",
        "Generic:",
        "Enum:",
        "Global Variable:",
    ];
    assert_eq!(module.len(), labels.len());
    for ((item, span), label) in module.iter().zip(labels) {
        let dump = item.dump();
        assert!(dump.starts_with(label), "unexpected dump: {dump}");
        assert!(!dump.contains("Position:"), "unexpected span: {dump}");
        let with_span = item.dump_with_span(*span);
        assert!(with_span.contains("Position:"));
    }
    let unit_dump = module[1].0.dump_with_span(module[1].1);
    assert!(unit_dump.contains("Name position:"));
    assert!(unit_dump.contains("Type position:"));
}

#[test]
fn rejects_self_outside_first_interface_parameter() {
    for source in [
        "func invalid(self) {}",
        "Point::func invalid(value:i32, self) {}",
        "generic G { func invalid(value:i32, mut self); };",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "accepted invalid receiver: {source}");
    }
}

#[test]
fn rejects_explicit_reference_syntax_for_method_receiver() {
    for source in [
        "Point::func set(self:$Point) {}",
        "Point::func set(mut self:$Point) {}",
        "Point::func set(&self) {}",
    ] {
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty(), "accepted invalid receiver: {source}");
    }
}
