use super::{func::function_process, FunctionBody, TempGlobalStmt, ImportModule, TempStmt, TokenIter, UnitDecl, UsingType};
use crate::lexer::TokenPack;
use crate::lexer::TokenPack::{Eof, Ident, Keyword, Operator};
use crate::lexer::token::Token;
use std::vec::IntoIter;
use crate::parser::split::next_and_assert;

fn variable_process(iter: &mut TokenIter)  {
    next_and_assert(iter,Token::Variable).unwrap();
}

fn keyword_process(
    iter: &mut TokenIter,
    keyword: Token,
) -> Result<TempStmt, TokenPack> {
    match keyword {
        Token::Variable => {}
        Token::Ident => {}
        Token::Match => {}
        Token::If => {}
        Token::While => {}
        Token::For => {}
        Token::Return => {}
        Token::Continue => {}
        Token::Break => {}
        Token::LBrace => {}
        _ => {}
    }
    todo!()
}

fn ident_process(
    iter: &mut TokenIter,
    ident: String,
) -> Result<TempStmt, TokenPack> {
    todo!()
}
pub fn statement_process(iter: &mut TokenIter) -> Result<TempStmt, TokenPack> {
    while let Some(token) = iter.next() {
        match token {
            Keyword(keyword) => {
                return keyword_process(iter, keyword);
            }
            Operator(operator) => {
                return Err(Operator(operator));
            }
            Ident(ident) => {
                return ident_process(iter, ident);
            }
            Eof => {}
        }
    }

    todo!()
}
