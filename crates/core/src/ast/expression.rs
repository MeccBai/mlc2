use crate::ast::symbols::Resolution;
pub mod cosnt_fold;
pub mod creator;
mod location;
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

#[derive(Debug, Clone)]
pub enum CompAtom {
    Poison(crate::parser::out::Span),
    VarValueA(Rc<Variable>, crate::parser::out::Span),
    CompositeA(Composite, crate::parser::out::Span),
    FuncCallA(FuncCall, crate::parser::out::Span),
    ConstValueA(ConstValue, crate::parser::out::Span),
    UnaryExprA(UnaryExpr, crate::parser::out::Span),
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
    pub pipeline: bool,
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

#[derive(Debug, Clone)]
pub enum Expression {
    Poison(crate::parser::out::Span),
    VarValueE(Rc<Variable>, crate::parser::out::Span),
    InitListE(InitialList, crate::parser::out::Span),
    CompositeE(Composite, crate::parser::out::Span),
    FuncCallE(FuncCall, crate::parser::out::Span),
    ConstValueE(ConstValue, crate::parser::out::Span),
    UnaryExprE(UnaryExpr, crate::parser::out::Span),
}
impl CompAtom {
    pub fn from_expr(expr: Expression) -> Self {
        let span = expr.span();
        match expr {
            Expression::Poison(..) => CompAtom::Poison(span),
            Expression::CompositeE(composite, ..) => CompAtom::CompositeA(composite, span),
            Expression::FuncCallE(func_call, ..) => CompAtom::FuncCallA(func_call, span),
            Expression::ConstValueE(const_value, ..) => CompAtom::ConstValueA(const_value, span),
            Expression::VarValueE(var, ..) => CompAtom::VarValueA(var, span),
            Expression::UnaryExprE(unary, ..) => CompAtom::UnaryExprA(unary, span),
            Expression::InitListE(_, ..) => ice("InitialList cannot be computed."),
        }
    }

    pub fn to_expression(self) -> Expression {
        let span = self.span();
        match self {
            CompAtom::Poison(..) => Expression::Poison(span),
            CompAtom::CompositeA(composite, ..) => Expression::CompositeE(composite, span),
            CompAtom::FuncCallA(func_call, ..) => Expression::FuncCallE(func_call, span),
            CompAtom::ConstValueA(const_value, ..) => Expression::ConstValueE(const_value, span),
            CompAtom::VarValueA(var, ..) => Expression::VarValueE(var, span),
            CompAtom::UnaryExprA(unary, ..) => Expression::UnaryExprE(unary, span),
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
        matches!(self, Self::Poison(..))
    }

    pub fn type_inference(&self, config: &mut Config, symbols: &mut dyn Resolution) -> TypeIndex {
        crate::TypeInfer::new(config, symbols).type_of(self)
    }

    pub fn is_null(&self) -> bool {
        match self {
            Expression::ConstValueE(const_value, ..) => const_value.ty.is_empty(),
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
