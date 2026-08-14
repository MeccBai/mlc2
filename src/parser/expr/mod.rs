use crate::ast::expr::operators::Operator;
use crate::lexer::TokenPack;
use crate::parser::TokenIter;

pub enum ExprAtom {
    Op(Operator),
    Val(String),
}

pub struct ExprFuncCall {
    pub name: String,
    pub args: Vec<Vec<ExprAtom>>,
}
pub fn expression_process(iter: &mut TokenIter) -> Result<Vec<ExprAtom>, TokenPack> {
    let mut expressions = Vec::<ExprAtom>::new();

    while let Some(token) = iter.next() {
        match token {
            _ => {}
        }
    }

    todo!()
}
