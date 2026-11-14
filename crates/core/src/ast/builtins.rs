//! Compiler-owned callable identities; these are not external function symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Todo,
    Alloc,
    Cast,
    Dealloc,
    ConstCStr,
    StrLen,
    Size,
    BitAnd,
    BitXor,
    BitOr,
    BitNot,
    BitShl,
    BitShr,
}

pub struct BuiltinDefinition {
    pub name: &'static str,
    pub kind: Builtin,
    pub type_arguments: usize,
    pub arguments: usize,
}

pub const FUNCTIONS: &[BuiltinDefinition] = &[
    BuiltinDefinition {
        name: "std::bit::and",
        kind: Builtin::BitAnd,
        type_arguments: 0,
        arguments: 2,
    },
    BuiltinDefinition {
        name: "std::bit::xor",
        kind: Builtin::BitXor,
        type_arguments: 0,
        arguments: 2,
    },
    BuiltinDefinition {
        name: "std::bit::or",
        kind: Builtin::BitOr,
        type_arguments: 0,
        arguments: 2,
    },
    BuiltinDefinition {
        name: "std::bit::not",
        kind: Builtin::BitNot,
        type_arguments: 0,
        arguments: 1,
    },
    BuiltinDefinition {
        name: "std::bit::shl",
        kind: Builtin::BitShl,
        type_arguments: 0,
        arguments: 2,
    },
    BuiltinDefinition {
        name: "std::bit::shr",
        kind: Builtin::BitShr,
        type_arguments: 0,
        arguments: 2,
    },
    BuiltinDefinition {
        name: "size",
        kind: Builtin::Size,
        type_arguments: 1,
        arguments: 0,
    },
    BuiltinDefinition {
        name: "todo",
        kind: Builtin::Todo,
        type_arguments: 0,
        arguments: 0,
    },
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

impl Builtin {
    pub fn bit_operator(self) -> Option<crate::ast::expression::operators::Operator> {
        use crate::ast::expression::operators::Operator;
        Some(match self {
            Self::BitAnd => Operator::BitAnd,
            Self::BitXor => Operator::BitXor,
            Self::BitOr => Operator::BitOr,
            Self::BitNot => Operator::BitNot,
            Self::BitShl => Operator::ShiftLeft,
            Self::BitShr => Operator::ShiftRight,
            _ => return None,
        })
    }
}

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

/// Constant-only intrinsics lower directly to typed literals, never call symbols.
pub(crate) fn numeric_constant(
    path: &[String],
) -> Option<(crate::ast::types::base_type::DataType, usize, bool, String)> {
    use crate::ast::types::base_type::DataType;
    let [namespace, ty, operation] = path else {
        return None;
    };
    if namespace != "std" {
        return None;
    }
    if let Some(bits) = ty
        .strip_prefix('i')
        .or_else(|| ty.strip_prefix('u'))
        .and_then(|bits| bits.parse::<usize>().ok())
    {
        if ![8, 16, 32, 64].contains(&bits)
            || !matches!(
                ty.as_str(),
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64"
            )
        {
            return None;
        }
        let signed = ty.starts_with('i');
        let value = match operation.as_str() {
            "max" => (1i128 << (bits - usize::from(signed))) - 1,
            "min" => {
                if signed {
                    -(1i128 << (bits - 1))
                } else {
                    0
                }
            }
            _ => return None,
        };
        return Some((DataType::Integer, bits, signed, value.to_string()));
    }
    let (bits, value) = match ty.as_str() {
        "f32" => (
            32,
            match operation.as_str() {
                "max" => f32::MAX as f64,
                "min" => f32::MIN as f64,
                "min_positive" => f32::MIN_POSITIVE as f64,
                "epsilon" => f32::EPSILON as f64,
                _ => return None,
            },
        ),
        "f64" => (
            64,
            match operation.as_str() {
                "max" => f64::MAX,
                "min" => f64::MIN,
                "min_positive" => f64::MIN_POSITIVE,
                "epsilon" => f64::EPSILON,
                _ => return None,
            },
        ),
        _ => return None,
    };
    Some((DataType::Float, bits, true, value.to_string()))
}

impl crate::ast::expression::Expression {
    pub fn is_todo(&self, symbols: &dyn crate::ast::symbols::Resolution) -> bool {
        matches!(self, Self::FuncCallE(call, ..) if call.callee.is_none() && matches!(call.func,crate::ast::symbols::EnumBool::False(i) if symbols.get_function_regular(i).builtin()==Some(Builtin::Todo)))
    }
}

impl crate::ast::expression::Expression {
    pub(crate) fn contextual_todo(
        &self,
        target: crate::ast::TypeIndex,
        symbols: &mut dyn crate::ast::symbols::Resolution,
    ) -> bool {
        if !self.is_todo(symbols) {
            return false;
        }
        if let Self::FuncCallE(call, ..) = self {
            if let crate::ast::symbols::EnumBool::False(index) = call.func {
                symbols.local_mut().functions.get_mut(index).ret_type = Some(target);
            }
        }
        true
    }
}

impl crate::ast::expression::Expression {
    pub fn contains_todo(&self, symbols: &dyn crate::ast::symbols::Resolution) -> bool {
        use crate::ast::expression::{Access, Expression as E, InitialList, UnaryExpr};
        if self.is_todo(symbols) {
            return true;
        }
        match self {
            E::VarValueE(v, ..) => v.init_val.contains_todo(symbols),
            E::CompositeE(c, ..) => c
                .members
                .iter()
                .any(|a| a.clone().to_expression().contains_todo(symbols)),
            E::FuncCallE(call, ..) => call.args.iter().any(|a| a.contains_todo(symbols)),
            E::UnaryExprE(UnaryExpr::Operator { value, .. }, ..) => {
                value.clone().to_expression().contains_todo(symbols)
            }
            E::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. }), ..) => {
                base.clone().to_expression().contains_todo(symbols)
            }
            E::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), ..) => {
                base.contains_todo(symbols) || index.contains_todo(symbols)
            }
            E::InitListE(
                InitialList::Array { values, .. } | InitialList::List { values, .. },
                ..,
            ) => values.iter().any(|a| a.contains_todo(symbols)),
            _ => false,
        }
    }
}
