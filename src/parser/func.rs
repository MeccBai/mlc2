use crate::lexer::TokenPack;
use crate::lexer::TokenPack::{Eof, Ident, Keyword, Operator};
use crate::lexer::token::Token;
use crate::parser::split::{ident_list_process, next_and_assert};
use crate::parser::{FunctionBody, TempStmt, TokenIter};
use std::vec::IntoIter;

fn func_param_process(iter: &mut TokenIter) -> Result<Vec<(String, String)>, TokenPack> {
    let mut params = Vec::new();
    next_and_assert(iter, Token::LParen)?;
    loop {
        let ty = match iter.next() {
            Some(Ident(v)) => v,
            Some(Keyword(Token::RParen)) => break,
            Some(other) => return Err(other),
            None => return Err(Eof),
        };
        let name = match iter.next() {
            Some(Ident(v)) => v,
            Some(other) => return Err(other),
            None => return Err(Eof),
        };
        params.push((name, ty));
        match iter.next() {
            Some(Keyword(Token::Comma)) => continue,
            Some(Keyword(Token::RParen)) => break,
            Some(other) => return Err(other),
            None => return Err(Eof),
        }
    }
    Ok(params)
}

fn func_ret_type_process(iter: &mut TokenIter) -> Result<Option<String>, TokenPack> {
    match iter.next() {
        Some(Operator(Token::Arrow)) => match iter.next() {
            Some(Ident(ty)) => Ok(Some(ty)),
            Some(other) => Err(other),
            None => Err(Eof),
        },
        Some(Keyword(Token::LBrace)) => {
            iter.next_back();
            Ok(None)
        }
        Some(other) => Err(other),
        None => Err(Eof),
    }
}

fn func_statement_process(iter: &mut TokenIter) -> Result<Vec<TempStmt>, TokenPack> {
    todo!()
}

fn func_generic_process(iter: &mut TokenIter) -> Result<Vec<String>, TokenPack> {
    next_and_assert(iter, Token::LAngle)?;
    let result = ident_list_process(iter, Token::LAngle, Token::RAngle);
    result
}

pub fn function_process(iter: &mut TokenIter) -> Result<FunctionBody, TokenPack> {
    let generics = func_generic_process(iter)?;
    let name = super::split::get_name(iter)?;
    let params = func_param_process(iter)?;
    let ret_type = func_ret_type_process(iter)?;
    let mut depth = 1;
    let mut body = Vec::new();
    next_and_assert(iter, Token::LBrace)?;
    while let Some(token) = iter.next() {
        match &token {
            Keyword(Token::LBrace) => {
                depth += 1;
            }
            Keyword(Token::RBrace) => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        body.push(token);
    }
    if depth != 0 {
        return Err(Eof);
    }
    Ok(FunctionBody {
        name,
        generics,
        params,
        ret_type,
        body,
        attributes: Vec::new(),
    })
}

#[test]
fn test_function_ident() {
    let code = r#"
        func<type1> test(type1 a) -> i8 {
            return a + 1;
        }
    "#;
    let tokens = crate::lexer::tokenize(code).unwrap();
    let mut iter = tokens.into_iter();
    next_and_assert(&mut iter, Token::Function).unwrap();
    let result = function_process(&mut iter);
    match result {
        Ok(func) => {
            println!("Function parsed successfully: {:?}", func.name);
            println!("Generics: {:?}", func.generics);
            println!("Params: {:?}", func.params);
            println!("Return type: {:?}", func.ret_type);
            println!("Body tokens: {:?}", func.body);
            println!()
        }
        Err(e) => {
            eprintln!("Function parsing failed: {:?}", e);
            panic!();
        }
    }
}
