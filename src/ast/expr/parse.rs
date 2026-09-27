use super::operators::Operator;
use crate::ast::SymbolTable;
use crate::ast::config::Config;
use crate::ast::expr::Expression;
use crate::parser::Spanned;
use crate::parser::out::{
    TempExpr,
    TempExpr::{Array, Binary, Call, Group, Init, Literal, Member, Path, Unary},
};

impl Expression {
    pub fn new(
        config: &mut Config,
        temp_var: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
    ) -> Self {
        let (expr, span) = temp_var;

        match expr {
            Literal { kind, text } => {}
            Path(path) => {}
            Unary { op, value } => {}
            Binary {
                operands,
                operators,
            } => {}
            Group(inner) => {}
            Call { callee, args } => {}
            Member {
                base,
                indirect,
                name,
            } => {}
            Init { target, values } => {}
            Array(values) => {}
        }

        Expression::null()
    }
}
