use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{TokenPack, token::Token};

use super::{
    MatchPattern, ParseError, Scope, Spanned, Statement,
    expr::expression_parser,
    out::Span,
    split::{ident, keyword, operator, spanned_ident, type_parser},
};

pub fn statement_parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Statement>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    recursive(|statement| {
        let expression = expression_parser();
        let semicolon = keyword(Token::Semicolon);

        let scope = statement
            .clone()
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace))
            .map(|statements| Scope { statements });

        let declared_type = operator(Token::Colon).ignore_then(type_parser()).or_not();
        let initializer = operator(Token::Assign).ignore_then(expression.clone());

        let variable = choice((
            keyword(Token::Variable).to(false),
            keyword(Token::Constant).to(true),
        ))
        .then(spanned_ident())
        .then(declared_type)
        .then(initializer)
        .then_ignore(semicolon.clone())
        .map(
            |(((constant, (name, name_span)), ty), value)| Statement::Variable {
                constant,
                name,
                name_span,
                ty,
                value: Some(value),
            },
        );

        let return_ = keyword(Token::Return)
            .ignore_then(expression.clone().or_not())
            .then_ignore(semicolon.clone())
            .map(Statement::Return);

        let break_ = keyword(Token::Break)
            .then_ignore(semicolon.clone())
            .to(Statement::Break);
        let continue_ = keyword(Token::Continue)
            .then_ignore(semicolon.clone())
            .to(Statement::Continue);

        let condition = expression
            .clone()
            .delimited_by(keyword(Token::LParen), keyword(Token::RParen));

        let if_ = keyword(Token::If)
            .ignore_then(condition.clone())
            .then(scope.clone())
            .then(keyword(Token::Else).ignore_then(scope.clone()).or_not())
            .map(|((condition, then_scope), else_scope)| Statement::If {
                condition,
                then_scope,
                else_scope,
            });

        let while_ = keyword(Token::While)
            .ignore_then(condition.clone())
            .then(scope.clone())
            .map(|(condition, scope)| Statement::While { condition, scope });

        let match_pattern = keyword(Token::Default)
            .to(MatchPattern::Default)
            .or(expression.clone().map(MatchPattern::Expression));
        let match_branch = match_pattern
            .then_ignore(keyword(Token::FatArrow))
            .then(scope.clone());
        let match_branches = match_branch
            .separated_by(operator(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace));
        let match_ = keyword(Token::Match)
            .ignore_then(condition)
            .then(match_branches)
            .map(|(value, branches)| Statement::Match { value, branches });

        let for_bounds = expression
            .clone()
            .then_ignore(operator(Token::Comma))
            .then(expression.clone())
            .delimited_by(keyword(Token::LeftBracket), keyword(Token::RightBracket));
        let for_ = keyword(Token::For)
            .ignore_then(spanned_ident())
            .then_ignore(keyword(Token::In))
            .then(for_bounds)
            .then(scope.clone())
            .map(
                |(((binding, binding_span), (start, end)), scope)| Statement::For {
                    binding,
                    binding_span,
                    start,
                    end,
                    scope,
                },
            );

        let assignment = expression
            .clone()
            .then_ignore(operator(Token::Assign))
            .then(expression.clone())
            .then_ignore(semicolon.clone())
            .map(|(target, value)| Statement::Assignment { target, value });

        let expression_statement = expression.then_ignore(semicolon).map(Statement::Expression);
        let anonymous = keyword(Token::Anonymous)
            .ignore_then(scope)
            .map(Statement::Anonymous);

        choice((
            variable,
            return_,
            break_,
            continue_,
            if_,
            while_,
            for_,
            match_,
            assignment,
            expression_statement,
            anonymous,
        ))
        .map_with(|statement, extra| (statement, extra.span()))
        .labelled("statement")
    })
    .boxed()
}

pub fn scope_parser<'tokens, I>()
-> impl Parser<'tokens, I, Scope, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    statement_parser()
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace))
        .map(|statements| Scope { statements })
        .labelled("scope")
        .boxed()
}
