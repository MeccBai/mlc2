use crate::ast::types::base_type::DataType;

pub struct SymbolName;

impl SymbolName {
    pub fn path(segments: &[String]) -> String {
        segments.join("::")
    }

    pub fn qualified(system_path: &[String], project_path: &str, name: &str) -> String {
        Self::member(&Self::prefix(system_path, project_path), name)
    }

    pub fn prefix(system_path: &[String], project_path: &str) -> String {
        system_path
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(project_path))
            .collect::<Vec<_>>()
            .join("::")
    }

    pub fn member(owner: &str, name: &str) -> String {
        format!("{owner}::{name}")
    }

    pub fn generic_instance(name: &str, arguments: &[String]) -> String {
        format!("{name}<{}>", arguments.join(","))
    }

    pub fn unconstrained_unit_param(unit: &str, param: &str) -> String {
        format!("\0unit::{}", Self::member(unit, param))
    }

    pub fn reference(base: &str, level: usize, mut_base: bool) -> String {
        let mutability = if mut_base { "mut " } else { "" };
        format!("{}{}{base}", "$".repeat(level), mutability)
    }

    pub fn array(element: &str, length: usize) -> String {
        format!("[{element},{length}]")
    }

    pub fn base_type(data_type: DataType, signed: bool, bits: usize) -> String {
        match data_type {
            DataType::Integer => format!("{}{bits}", if signed { "i" } else { "u" }),
            DataType::Float => format!("f{bits}"),
            DataType::Boolean => "bool".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SymbolName;
    use crate::ast::types::base_type::DataType;

    #[test]
    fn symbol_forms_are_consistent() {
        assert_eq!(
            SymbolName::qualified(&["sys".into()], "project", "Point"),
            "sys::project::Point"
        );
        assert_eq!(SymbolName::member("Point", "get"), "Point::get");
        assert_eq!(
            SymbolName::generic_instance("Box", &["i32".into()]),
            "Box<i32>"
        );
        assert_eq!(SymbolName::reference("i32", 2, false), "$$i32");
        assert_eq!(SymbolName::reference("i32", 2, true), "$$mut i32");
        assert_eq!(SymbolName::array("i32", 4), "[i32,4]");
        assert_eq!(SymbolName::base_type(DataType::Integer, true, 32), "i32");
    }
}
