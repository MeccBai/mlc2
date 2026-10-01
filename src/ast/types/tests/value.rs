use crate::ast::symbols::{EnumBool, SymbolTable};
use crate::ast::types::base_type::DataType;
use crate::ast::types::{CompileType, ValueType};

#[test]
fn type_entity_owns_value_qualifier_and_index_can_convert() {
    let mut symbols = SymbolTable::new();
    let flex = symbols.get_base(DataType::Integer, 32, true);
    let final_type = flex.into(ValueType::Final, &mut symbols.types);
    let constant = final_type.into(ValueType::Constant, &mut symbols.types);

    assert_ne!(flex, final_type);
    assert_ne!(final_type, constant);
    assert_eq!(final_type.into(ValueType::Flex, &mut symbols.types), flex);
    assert_eq!(final_type.format(&symbols.types), "val i32");
    assert_eq!(constant.dump(&symbols.types), "const i32");
    assert!(matches!(
        symbols.types.get(final_type),
        CompileType::Qualified {
            value: ValueType::Final,
            ..
        }
    ));
    assert!(flex.type_check(false, &final_type, &symbols.types));
}

#[test]
fn immutable_reference_dereferences_to_final_value() {
    let mut symbols = SymbolTable::new();
    let base = symbols.get_base(DataType::Integer, 32, true);
    let reference = base.make_ref(&mut symbols.types, false);
    assert_eq!(
        reference
            .deref(&mut symbols.types)
            .unwrap()
            .value_type(&symbols.types),
        ValueType::Final
    );
    let mutable = base.make_ref(&mut symbols.types, true);
    assert_eq!(
        mutable
            .deref(&mut symbols.types)
            .unwrap()
            .value_type(&symbols.types),
        ValueType::Flex
    );
}
