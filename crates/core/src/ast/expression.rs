use crate::ast::symbols::Resolution;
pub mod cosnt_fold;
pub mod creator;
mod numeric_initialization;
pub mod operators;
mod range_check;

use std::rc::Rc;

use crate::ast::arena::{InterfaceIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::statement::Variable;
use crate::ast::types::CompileType::{self, List};
use crate::ast::types::ListType;
use crate::ast::types::ValueType;
use crate::ast::types::base_type::DataType;
use crate::ast::{
    FuncIndex, TypeIndex,
    symbols::{EnumBool, SymbolTable},
};
use crate::diagnostic::ice::ice;
use crate::parser::out::TempLiteralKind;

use operators::Operator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompAtom {
    Poison,
    VarValueA(Rc<Variable>),
    CompositeA(Composite),
    FuncCallA(FuncCall),
    ConstValueA(ConstValue),
    UnaryExprA(UnaryExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Composite {
    pub members: Vec<CompAtom>,
    pub operators: Vec<Operator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitialList {
    List {
        onwer: Option<TypeIndex>,
        values: Vec<Expression>,
    },
    Array {
        ty: TypeIndex,
        values: Vec<Expression>,
    },
    //BaseArray {
    //    ty: TypeIndex,
    //    values: Vec<Expression>,
    //},
    String {
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncCall {
    pub callee: Option<Box<Expression>>,
    pub func: EnumBool<InterfaceIndex, FuncIndex>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumValue {
    pub enum_type: TypeIndex,
    pub value: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstValue {
    pub value: String,
    pub ty: TypeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Member {
        base: Box<CompAtom>,
        indirect: bool,
        name: String,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
}

impl ConstValue {
    pub fn null() -> Self {
        ConstValue {
            value: String::new(),
            ty: TypeIndex::empty(),
        }
    }

    pub fn new(value: (TempLiteralKind, String), symbols: &dyn Resolution) -> Self {
        let (kind, text) = value;

        let ty = match kind {
            TempLiteralKind::Integer => symbols.get_base(DataType::Integer, 32, true),
            TempLiteralKind::Float => symbols.get_base(DataType::Float, 64, true),
            TempLiteralKind::String => ice(
                "String literal type cannot be a base type, it must be a reference to a string type!",
            ),
            TempLiteralKind::Boolean => symbols.get_base(DataType::Boolean, 8, false),
            TempLiteralKind::Null => TypeIndex::empty(),
        };

        ConstValue { value: text, ty }
    }
}

impl InitialList {
    pub fn empty() -> Self {
        Self::Array {
            ty: TypeIndex::empty(),
            values: Vec::new(),
        }
    }

    pub fn from_string(text: String) -> Self {
        Self::String { value: text }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryExpr {
    Function { function: FuncIndex, ty: TypeIndex },
    Operator { op: Operator, value: Box<CompAtom> },
    Access(Access),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expression {
    Poison,
    VarValueE(Rc<Variable>),
    InitListE(InitialList),
    CompositeE(Composite),
    FuncCallE(FuncCall),
    ConstValueE(ConstValue),
    UnaryExprE(UnaryExpr),
}
impl CompAtom {
    pub fn from_expr(expr: Expression) -> Self {
        match expr {
            Expression::Poison => CompAtom::Poison,
            Expression::CompositeE(composite) => CompAtom::CompositeA(composite),
            Expression::FuncCallE(func_call) => CompAtom::FuncCallA(func_call),
            Expression::ConstValueE(const_value) => CompAtom::ConstValueA(const_value),
            Expression::VarValueE(var) => CompAtom::VarValueA(var),
            Expression::UnaryExprE(unary) => CompAtom::UnaryExprA(unary),
            Expression::InitListE(_) => ice("InitialList cannot be computed."),
        }
    }

    pub fn to_expression(self) -> Expression {
        match self {
            CompAtom::Poison => Expression::Poison,
            CompAtom::CompositeA(composite) => Expression::CompositeE(composite),
            CompAtom::FuncCallA(func_call) => Expression::FuncCallE(func_call),
            CompAtom::ConstValueA(const_value) => Expression::ConstValueE(const_value),
            CompAtom::VarValueA(var) => Expression::VarValueE(var),
            CompAtom::UnaryExprA(unary) => Expression::UnaryExprE(unary),
        }
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut dyn Resolution) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).atom_type(self)
    }

    pub fn is_condition(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        self.clone().to_expression().is_condition(config, symbols)
    }

    pub fn is_left_value(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        crate::TypeInfer::new(config, symbols).atom_assignable(self)
    }

    pub fn is_const(&self, symbols: &dyn Resolution) -> bool {
        crate::TypeInfer::atom_is_constant(self, symbols)
    }
}

impl Expression {
    pub fn is_poisoned(&self) -> bool {
        matches!(self, Self::Poison)
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut dyn Resolution) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).type_of(self)
    }

    pub fn is_null(&self) -> bool {
        match self {
            Expression::ConstValueE(const_value) => const_value.ty.is_empty(),
            _ => false,
        }
    }

    pub fn type_check(
        &self,
        target: &TypeIndex,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> bool {
        crate::TypeInfer::new(config, symbols).compatible(self, *target)
    }

    pub fn is_condition(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        let ty = self.type_inference(config, symbols);
        if ty.is_empty() {
            return false;
        }
        let boolean = symbols.get_base(DataType::Boolean, 8, true);
        ty.type_check(false, &boolean, symbols)
    }

    pub fn assignable(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        crate::TypeInfer::new(config, symbols).assignable(self)
    }

    pub fn is_const(&self, symbols: &dyn Resolution) -> bool {
        crate::TypeInfer::is_constant(self, symbols)
    }

    pub fn ref_calculate_check(&self) -> bool {
        true
    }
}

impl Composite {
    pub(crate) fn type_inference(
        &self,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).composite_type(self)
    }
}

impl FuncCall {
    pub(crate) fn type_inference(&self, symbols: &dyn Resolution) -> TypeIndex {
        crate::TypeInfer::call_result(self, symbols)
    }
}

impl UnaryExpr {
    pub(crate) fn type_inference(
        &self,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).unary_type(self)
    }

    pub(crate) fn is_left_value(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        crate::TypeInfer::new(config, symbols).unary_assignable(self)
    }

    pub(crate) fn is_const(&self, symbols: &dyn Resolution) -> bool {
        crate::TypeInfer::unary_is_constant(self, symbols)
    }
}

impl Access {
    pub(crate) fn type_inference(
        &self,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).access_type(self)
    }

    pub(crate) fn is_left_value(&self, config: &mut Config, symbols: &mut dyn Resolution) -> bool {
        crate::TypeInfer::new(config, symbols).access_assignable(self)
    }

    pub(crate) fn is_const(&self, symbols: &dyn Resolution) -> bool {
        crate::TypeInfer::access_is_constant(self, symbols)
    }
}
