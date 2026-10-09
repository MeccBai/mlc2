use super::*;
use crate::ast::{
    AnalyzedAst, arena::FuncIndex, generic::GenericRequire, symbols::EnumBool, types::ValueType,
};
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::TempPath;

fn config() -> Config {
    Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    )
}

#[test]
fn resource_identity_is_distinct_and_dereferencing_does_not_transfer_ownership() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let plain = resolve_type_with_bindings(
        &mut config,
        reference("i32", true, 2),
        &mut symbols,
        &HashMap::<String, TypeIndex>::new(),
    )
    .unwrap();
    let resource = resolve_type_with_bindings(
        &mut config,
        (
            TempType::Resource {
                inner: Box::new(reference("i32", true, 2)),
            },
            (0..8).into(),
        ),
        &mut symbols,
        &HashMap::<String, TypeIndex>::new(),
    )
    .unwrap();
    assert_ne!(plain, resource);
    assert!(resource.is_resource(&symbols));
    assert!(!plain.is_resource(&symbols));
    assert_eq!(resource.format(&symbols), "res $$mut i32");
    assert_eq!(resource.size(&symbols), plain.size(&symbols));
    assert!(!resource.type_check(false, &plain, &symbols));
    assert!(plain.type_check(false, &resource, &symbols));
    let child = resource.deref(&mut symbols).unwrap();
    assert!(!child.is_resource(&symbols));
    assert_eq!(child.format(&symbols), "$mut i32");
}

fn reference(name: &str, mutable: bool, level: usize) -> Spanned<TempType> {
    let span = (4..8).into();
    let mut ty = (
        TempType::Path(TempPath {
            segments: vec![name.into()],
        }),
        span,
    );
    for _ in 0..level {
        ty = (
            TempType::Reference {
                inner: Box::new(ty),
                mutable,
            },
            span,
        );
    }
    ty
}

#[test]
fn resource_generic_instantiation_preserves_ownership() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let generic = symbols
        .generics
        .requires
        .insert("test::T".into(), GenericRequire::empty("test::T".into()));
    let context = HashMap::from([("T".to_owned(), generic)]);
    let resource = resolve_type(
        &mut config,
        (
            TempType::Resource {
                inner: Box::new(reference("T", true, 1)),
            },
            (0..10).into(),
        ),
        &mut symbols,
        Some(&context),
    )
    .unwrap();
    let concrete = symbols.get_base(crate::ast::types::base_type::DataType::Integer, 32, true);
    let instance = resource
        .instantiation(
            &mut config,
            &HashMap::from([(generic, concrete)]),
            &mut symbols,
            None,
            (0..10).into(),
        )
        .unwrap();
    assert!(instance.is_resource(&symbols));
    assert_eq!(instance.format(&symbols), "res $mut i32");
}

#[test]
fn reference_identity_includes_generic_owner_mutability_and_depth() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let first = symbols
        .generics
        .requires
        .insert("first::T".into(), GenericRequire::empty("first::T".into()));
    let second = symbols.generics.requires.insert(
        "second::T".into(),
        GenericRequire::empty("second::T".into()),
    );
    let first_context = HashMap::from([("T".to_owned(), first)]);
    let second_context = HashMap::from([("T".to_owned(), second)]);
    let a = resolve_type(
        &mut config,
        reference("T", false, 1),
        &mut symbols,
        Some(&first_context),
    )
    .unwrap();
    let b = resolve_type(
        &mut config,
        reference("T", false, 1),
        &mut symbols,
        Some(&second_context),
    )
    .unwrap();
    let mutable = resolve_type(
        &mut config,
        reference("T", true, 1),
        &mut symbols,
        Some(&first_context),
    )
    .unwrap();
    let nested = resolve_type(
        &mut config,
        reference("T", false, 2),
        &mut symbols,
        Some(&first_context),
    )
    .unwrap();
    assert_ne!(a, b);
    assert_ne!(a, mutable);
    assert_ne!(a, nested);
    assert_eq!(
        a,
        resolve_type(
            &mut config,
            reference("T", false, 1),
            &mut symbols,
            Some(&first_context)
        )
        .unwrap()
    );
    assert!(symbols.types.get_by_name(&String::new()).is_none());
    let CompileType::Ref(reference) = symbols.types.get(nested) else {
        panic!("reference expected");
    };
    assert_eq!(reference.level, 2);
    assert!(
        matches!(symbols.types.get(reference.base), CompileType::Generic(index) if *index == first)
    );
    assert!(!nested.format(&symbols.types).is_empty());
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn statement_context_resolves_symbolic_and_concrete_generic_types() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let generic = symbols
        .generics
        .requires
        .insert("T".into(), GenericRequire::empty("T".into()));
    let mut context = StatementContext::new(EnumBool::False(FuncIndex::empty()));
    assert!(context.insert_generic_parameter(&mut config, "T".into(), generic, (0..1).into()));
    let symbolic = resolve_type_with_bindings(
        &mut config,
        reference("T", false, 1),
        &mut symbols,
        &context,
    )
    .unwrap();
    assert!(symbolic.is_generic(&symbols.types));
    let concrete = symbols.types.get_by_name(&"i32".into()).unwrap();
    context.bind_generic_type("T", concrete);
    let resolved = resolve_type(
        &mut config,
        reference("T", false, 1),
        &mut symbols,
        Some(&context),
    )
    .unwrap();
    assert_eq!(resolved.format(&symbols.types), "$i32");
    assert!(!resolved.is_generic(&symbols.types));
}

#[test]
fn qualified_generic_reference_can_be_named_and_instantiated() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let generic = symbols
        .generics
        .requires
        .insert("T".into(), GenericRequire::empty("T".into()));
    let ty = symbols.types.insert(
        SymbolName::generic_type(generic),
        CompileType::Generic(generic),
    );
    let ty = ty.into_value_type(ValueType::Final, &mut symbols.types);
    let context = HashMap::from([("T".to_owned(), ty)]);
    let resolved = resolve_type(
        &mut config,
        reference("T", false, 2),
        &mut symbols,
        Some(&context),
    )
    .unwrap();
    let concrete = symbols.types.get_by_name(&"i32".into()).unwrap();
    let params = HashMap::from([(generic, concrete)]);
    let instantiated = resolved
        .instantiation(&mut config, &params, &mut symbols, None, (0..1).into())
        .unwrap();
    assert!(instantiated.format(&symbols.types).contains("i32"));
    assert!(!instantiated.is_generic(&symbols.types));
}

#[test]
fn function_and_unit_contexts_resolve_reference_members_without_empty_keys() {
    let source = "unit Box<T,U> { first:$T; second:$mut U; nested:$$T; }; func<T> identity(value:$T) -> $T { return value; } func main() { var x = 1; var p = identity<i32>(@x); }";
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let mut ast = AnalyzedAst::new(config(), module.unwrap());
    assert!(
        ast.config.error_handle().errors.is_empty(),
        "{:?}",
        ast.config.error_handle().errors
    );
    let unit_index = ast
        .symbols
        .generics
        .units
        .get_by_name(&ast.config.symbol_name("Box"))
        .unwrap();
    let unit = ast.symbols.generics.units.get(unit_index).clone();
    let function_index = ast
        .symbols
        .generics
        .functions
        .get_by_name(&"identity".into())
        .unwrap();
    let function = ast.symbols.generics.functions.get(function_index).clone();
    let unit_ref = resolve_type(
        &mut ast.config,
        reference("T", false, 1),
        &mut ast.symbols,
        Some(&unit),
    )
    .unwrap();
    let function_ref = resolve_type(
        &mut ast.config,
        reference("T", false, 1),
        &mut ast.symbols,
        Some(&function),
    )
    .unwrap();
    assert_eq!(unit_ref, unit.members[0].member_type);
    assert_eq!(function_ref, function.params[0].0);
    assert_ne!(unit_ref, function_ref);
    assert_eq!(ast.symbols.function_instances.len(), 1);
    assert!(ast.symbols.types.get_by_name(&String::new()).is_none());
}
