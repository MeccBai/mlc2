use super::*;
use crate::ast::{
    config::FileId,
    types::{ListType, RefType, UnitType, ValueType, base_type::DataType, unit_type::UnitMember},
};

#[test]
fn nested_queries_follow_each_indices_file_id() {
    let first_id = FileId::new(101);
    let second_id = FileId::new(102);
    let first = SymbolTable::for_file(first_id);
    let integer = first.get_base(DataType::Integer, 32, true);
    let expected_size = integer.size(&first.types);
    let expected_align = integer.align(&first.types);
    let mut second = SymbolTable::for_file(second_id);
    let list = second.types.insert(
        "foreign-array".into(),
        CompileType::List(ListType::new(integer, 3)),
    );
    let reference = second.types.insert(
        "foreign-ref".into(),
        CompileType::Ref(RefType::new(integer, 1, false)),
    );
    let qualified = second.types.insert(
        "foreign-qualified".into(),
        CompileType::Qualified {
            base: Box::new(CompileType::List(ListType::new(integer, 3))),
            value: ValueType::Final,
        },
    );
    let mut unit = UnitType::empty();
    unit.name = "Owner".into();
    unit.members.push(UnitMember {
        name: "values".into(),
        member_type: list,
        public: true,
    });
    let unit = second.types.insert("Owner".into(), CompileType::Unit(unit));
    let mut store = ArenaStore::new();
    store.register(first_id, first).unwrap();
    store.register(second_id, second).unwrap();
    assert_eq!(integer.format(&store), "i32");
    assert_eq!(list.format(&store), "[i32,3]");
    assert_eq!(list.dump(&store), "[i32,3]");
    assert_eq!(list.size(&store), expected_size * 3);
    assert_eq!(list.align(&store), expected_align);
    assert_eq!(reference.format(&store), "$i32");
    assert_eq!(qualified.format(&store), "val [i32,3]");
    assert_eq!(qualified.size(&store), expected_size * 3);
    assert_eq!(unit.size(&store), expected_size * 3);
    assert_eq!(unit.align(&store), expected_align);
    assert_eq!(unit.format(&store), "Owner");
    assert!(unit.dump(&store).contains("values"));
}
