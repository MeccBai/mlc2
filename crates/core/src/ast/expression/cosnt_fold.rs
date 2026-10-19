use super::{
    Access, CompAtom, Composite, ConstValue, Expression, InitialList, UnaryExpr,
    operators::Operator,
};
use crate::ast::symbols::Resolution;
use crate::ast::{
    config::Config,
    symbols::SymbolTable,
    types::{CompileType, ValueType, base_type::DataType},
};

impl Expression {
    pub fn const_fold(self, config: &mut Config, symbols: &dyn Resolution) -> Self {
        crate::TypeInfer::read_only(config, symbols).constant(self)
    }
}
#[cfg(test)]
mod tests;
