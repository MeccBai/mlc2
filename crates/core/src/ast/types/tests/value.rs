use crate::ast::symbols::{EnumBool, SymbolTable};
use crate::ast::types::CompileType;
use crate::ast::types::base_type::DataType;

#[test]
fn type_entity_only_tracks_immutability_and_index_can_convert() {
    let mut symbols = SymbolTable::new();
    let flex = symbols.get_base(DataType::Integer, 32, true);
    let final_type = flex.with_immutability(true, &mut symbols.types);
    let immutable_again = final_type.with_immutability(true, &mut symbols.types);

    assert_ne!(flex, final_type);
    assert_eq!(final_type, immutable_again);
    assert_eq!(
        final_type.with_immutability(false, &mut symbols.types),
        flex
    );
    assert_eq!(final_type.format(&symbols.types), "val i32");
    assert_eq!(immutable_again.dump(&symbols.types), "val i32");
    assert!(matches!(
        symbols.types.get(final_type),
        CompileType::Qualified { immut: true, .. }
    ));
    assert!(flex.type_check(false, &final_type, &symbols.types));
}

#[test]
fn immutable_reference_dereferences_to_immutable_value() {
    let mut symbols = SymbolTable::new();
    let base = symbols.get_base(DataType::Integer, 32, true);
    let reference = base.make_ref(&mut symbols.types, false);
    assert_eq!(
        reference
            .deref(&mut symbols.types)
            .unwrap()
            .is_immutable(&symbols.types),
        true
    );
    let mutable = base.make_ref(&mut symbols.types, true);
    assert_eq!(
        mutable
            .deref(&mut symbols.types)
            .unwrap()
            .is_immutable(&symbols.types),
        false
    );
}

#[test]
fn pointer_size_and_alignment_follow_config_for_all_type_queries() {
    use crate::ast::{config::Config, function::FuncSymbol, types::FunctionType};
    use crate::diagnostic::{error::ErrorHandle, warning::WarningHandle};
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        vec![],
        String::new(),
        String::new(),
        ErrorHandle::new(String::new()),
        WarningHandle::new(String::new()),
    );
    config.set_target_layout(crate::ast::config::TargetLayout {
        ptr_size: 4,
        ptr_align: 4,
    });
    assert_eq!((config.ptr_size, config.ptr_align), (4, 4));
    let mut symbols = SymbolTable::new();
    symbols.types.layout = config.target_layout();
    let integer = symbols.get_base(DataType::Integer, 32, true);
    let reference = integer.make_ref(&mut symbols.types, false);
    let function = FunctionType::declare(vec![], Some(integer), false, &mut symbols);
    for ty in [reference, function] {
        assert_eq!(ty.size(&symbols), 4);
        assert_eq!(ty.align(&symbols), 4);
        assert_eq!(symbols.types.get(ty).size(&symbols), 4);
    }
    config.ptr_size = 16;
    config.ptr_align = 16;
    symbols.types.layout = config.target_layout();
    for ty in [reference, function] {
        assert_eq!(ty.size(&symbols), 16);
        assert_eq!(ty.align(&symbols), 16);
    }
    assert_eq!(
        reference
            .with_immutability(true, &mut symbols.types)
            .size(&symbols),
        16
    );
}
