use super::{
    FunctionBody, TempGlobalStmt, ImportModule, TempStmt, TokenIter, UnitDecl, UsingType,
    func::function_process,
};
use crate::lexer::TokenPack;
use crate::lexer::TokenPack::{Eof, Ident};
use crate::lexer::token::Token;

pub(super) fn ident_list_process(
    iter: &mut TokenIter,
    separator: Token,
    end: Token,
) -> Result<Vec<String>, TokenPack> {
    let mut result = Vec::new();
    let mut expect_ident = true;

    while let Some(token_pack) = iter.next() {
        match token_pack {
            Ident(ident) if expect_ident => {
                result.push(ident);
                expect_ident = false;
            }
            TokenPack::Keyword(token) if token == separator && !expect_ident => {
                expect_ident = true;
            }
            TokenPack::Operator(token) if token == separator && !expect_ident => {
                expect_ident = true;
            }
            TokenPack::Keyword(token) if token == end && !expect_ident => {
                break;
            }
            TokenPack::Operator(token) if token == end && !expect_ident => {
                break;
            }
            other => {
                return Err(other);
            }
        }
    }

    Ok(result)
}

pub fn get_name(iter: &mut TokenIter) -> Result<String, TokenPack> {
    match iter.next() {
        Some(Ident(ident)) => Ok(ident),
        Some(other) => Err(other),
        None => Err(Eof),
    }
}

fn import_process(iter: &mut TokenIter) -> Result<TempGlobalStmt, TokenPack> {
    let path = ident_list_process(iter, Token::ColonColon, Token::Semicolon)?;
    Ok(TempGlobalStmt::Import(ImportModule::new(path)))
}

fn attribute_process(iter: &mut TokenIter) -> Result<Vec<String>, TokenPack> {
    ident_list_process(iter, Token::Comma, Token::AttributeEnd)
}

pub(super) fn next_and_assert(iter: &mut TokenIter, expected: Token) -> Result<(), TokenPack> {
    match iter.next() {
        Some(TokenPack::Keyword(token)) if token == expected => Ok(()),
        Some(TokenPack::Operator(token)) if token == expected => Ok(()),
        Some(other) => Err(other),
        None => Err(Eof),
    }
}

pub(super) fn split_by_semicolon(iter: &mut TokenIter,end: Token) -> Result<Vec<Vec<TokenPack>>, TokenPack> {
    let mut result = Vec::new();
    let mut current = Vec::new();

    while let Some(token_pack) = iter.next() {
        match token_pack {
            TokenPack::Keyword(token) if token == end => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                    current = Vec::new();
                }
                break;
            }
            TokenPack::Operator(token) if token == end => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                    current = Vec::new();
                }
                break;
            }
            TokenPack::Keyword(token) if token == Token::Semicolon => {
                result.push(std::mem::take(&mut current));
                current = Vec::new();
            }
            other => {
                current.push(other);
            }
        }
    }

    if !current.is_empty() {
        result.push(std::mem::take(&mut current));
    }

    Ok(result)
}

pub fn coarse_segmentate(tokens: Vec<TokenPack>) -> Result<Vec<TempGlobalStmt>, TokenPack> {
    let mut statements = Vec::<TempGlobalStmt>::new();
    let mut iter = tokens.into_iter();

    let mut attributes = Vec::<String>::new();

    while let Some(token_pack) = iter.next() {
        match token_pack {
            TokenPack::Keyword(keyword) => match keyword {
                Token::Import => {
                    let result = import_process(&mut iter);
                    match result {
                        Ok(statement) => statements.push(statement),
                        Err(token_pack) => return Err(token_pack),
                    }
                }
                Token::AttributeStart => {
                    let result = attribute_process(&mut iter);
                    match result {
                        Ok(attr) => attributes = attr,
                        Err(token_pack) => return Err(token_pack),
                    }
                }
                Token::Function => {
                    let result = function_process(&mut iter);
                    match result {
                        Ok(mut func) => {
                            func.attributes = std::mem::take(&mut attributes);
                            statements.push(TempGlobalStmt::Function(func));
                            attributes = Vec::<String>::new();
                        }
                        Err(token_pack) => return Err(token_pack),
                    }
                }
                Token::Generic => {}
                Token::Unit => {}
                Token::Using => {}
                Token::Global => {}
                Token::Enum => {}
                _ => {}
            },
            Ident(ident) => {
                return Err(Ident(ident));
            }
            TokenPack::Operator(_) => {
                return Err(token_pack);
            }
            Eof => {
                break;
            }
        }
    }
    Ok(statements)
}
