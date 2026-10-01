use super::SymbolName;
use crate::ast::types::base_type::DataType;

#[test]
fn symbol_forms_are_consistent() {
    assert_eq!(
        SymbolName::qualified(&["sys".into()], "project", "Point"),
        "sys::project::Point"
    );
    assert_eq!(SymbolName::member("Point", "get"), "Point::get");
    assert_eq!(SymbolName::callable(None, "get"), "get");
    assert_eq!(SymbolName::callable(Some("Point"), "get"), "Point::get");
    assert_eq!(
        SymbolName::generic_instance("Box", &["i32".into()]),
        "Box<i32>"
    );
    assert_eq!(SymbolName::reference("i32", 2, false), "$$i32");
    assert_eq!(SymbolName::reference("i32", 2, true), "$$mut i32");
    assert_eq!(SymbolName::array("i32", 4), "[i32,4]");
    assert_eq!(SymbolName::base_type(DataType::Integer, true, 32), "i32");
}
