//! Compiler-owned callable identities; these are not external function symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Alloc,
    Cast,
    Dealloc,
    ConstCStr,
    StrLen,
}

pub struct BuiltinDefinition {
    pub name: &'static str,
    pub kind: Builtin,
    pub type_arguments: usize,
    pub arguments: usize,
}

pub const FUNCTIONS: &[BuiltinDefinition] = &[
    BuiltinDefinition {
        name: "str::len",
        kind: Builtin::StrLen,
        type_arguments: 0,
        arguments: 1,
    },
    BuiltinDefinition {
        name: "std::mem::alloc",
        kind: Builtin::Alloc,
        type_arguments: 1,
        arguments: 1,
    },
    BuiltinDefinition {
        name: "cast",
        kind: Builtin::Cast,
        type_arguments: 1,
        arguments: 1,
    },
    BuiltinDefinition {
        name: "std::mem::dealloc",
        kind: Builtin::Dealloc,
        type_arguments: 1,
        arguments: 1,
    },
    BuiltinDefinition {
        name: "const_c_str",
        kind: Builtin::ConstCStr,
        type_arguments: 0,
        arguments: 1,
    },
];

/// Memory intrinsics are declarations in an imported standard-library module.
pub fn module_builtin(module: &str, name: &str) -> Option<Builtin> {
    lookup(&format!("{module}::{name}")).map(|definition| definition.kind)
}

pub fn lookup(name: &str) -> Option<&'static BuiltinDefinition> {
    FUNCTIONS.iter().find(|definition| definition.name == name)
}

impl crate::ast::function::FuncSymbol {
    pub fn builtin(&self) -> Option<Builtin> {
        self.attributes
            .iter()
            .find_map(|attribute| match attribute {
                crate::ast::attribute::FuncAttibute::Builtin(kind) => Some(*kind),
                _ => None,
            })
    }
}

impl crate::ast::expression::FuncCall {
    pub(crate) fn is_const(&self, symbols: &dyn crate::ast::symbols::Resolution) -> bool {
        crate::TypeInfer::call_is_constant(self, symbols)
    }
}
