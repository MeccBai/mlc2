use super::*;
use crate::ast::{config::FileId, symbols::PackageSymbolTable, types::BaseType};

#[test]
fn cross_arena_references_compare_types_and_preserve_reference_restrictions() {
    let mut package = PackageSymbolTable::new();
    let mut bases = vec![];
    for id in [FileId::new(0), FileId::new(1)] {
        package.register(id).unwrap();
        let ty = BaseType::base_types()[0].1.clone();
        bases.push(
            package
                .file_mut(id)
                .unwrap()
                .types
                .insert("test-i8".into(), ty),
        );
    }
    assert_ne!(bases[0], bases[1]);
    let expected = RefType::new(bases[0], 1, false);
    assert!(expected.type_check(&RefType::new(bases[1], 1, false), &package));
    assert!(expected.type_check(&RefType::new(bases[1], 1, true), &package));
    assert!(!expected.type_check(&RefType::new(bases[1], 2, false), &package));
    assert!(
        !RefType::new(bases[0], 1, true).type_check(&RefType::new(bases[1], 1, false), &package)
    );
    let different = package
        .file_mut(FileId::new(1))
        .unwrap()
        .types
        .insert("test-i16".into(), BaseType::base_types()[1].1.clone());
    assert!(!expected.type_check(&RefType::new(different, 1, false), &package));
}
