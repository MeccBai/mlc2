use crate::ast::types::base_type::DataType;

pub struct SymbolName;

impl SymbolName {
    pub fn function_signature(signature: &str) -> String {
        format!(".mlc.signature.{signature}")
    }
    pub fn deconstruct(symbol_name: &str) -> String {
        Self::member(symbol_name, "deconstruct")
    }
    pub fn global_initializer(file: crate::ast::config::FileId) -> String {
        format!(".mlc.init.{}", file.index())
    }

    /// A callable without an owner stays in the compilation unit's namespace.
    pub fn callable(owner: Option<&str>, name: &str) -> String {
        match owner {
            Some(owner) => Self::member(owner, name),
            None => name.to_owned(),
        }
    }

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

    pub fn builtin(name: &str, target: &str, source: &str) -> String {
        format!(".mlc.builtin.{name}<{target},{source}>")
    }

    pub fn module_initializer(module: &str) -> String {
        use std::fmt::Write;
        let mut name = String::from("__mlc__");
        for byte in module.bytes() {
            write!(name, "{byte:02x}").expect("String write");
        }
        name.push_str("__global__init");
        name
    }

    pub fn generic_instance(name: &str, arguments: &[String]) -> String {
        format!("{name}<{}>", arguments.join(","))
    }

    pub fn unconstrained_unit_param(unit: &str, param: &str) -> String {
        format!("\0unit::{}", Self::member(unit, param))
    }

    pub fn generic_param(kind: &str, owner: &str, param: &str) -> String {
        format!("\0{kind}::{}", Self::member(owner, param))
    }

    pub fn generic_type(index: crate::ast::GenericIndex) -> String {
        format!("\0generic:{index:?}")
    }

    pub fn reference(base: &str, level: usize, mut_base: bool) -> String {
        let mutability = if mut_base { "mut " } else { "" };
        format!("{}{}{base}", "$".repeat(level), mutability)
    }

    pub fn resource(reference: &str) -> String {
        format!("res {reference}")
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
mod tests;
