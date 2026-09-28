use chumsky::{input::ValueInput, prelude::*};

use crate::ast::expression::operators::Operator;
use crate::lexer::{TokenPack, token::Token};

use super::{
    Expr, LiteralKind, ParseError, Spanned,
    out::Span,
    split::{keyword, operator, path, spanned_ident, type_parser},
};

enum Postfix {
    Call(Vec<Spanned<Expr>>),
    Member(bool, Spanned<String>),
    Index(Spanned<Expr>),
}

fn binary<'tokens, I>(
    token: Token,
    op: Operator,
) -> impl Parser<'tokens, I, Operator, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    operator(token).to(op)
}

fn fold_binary<'tokens, I, P, O>(
    operand: P,
    op: O,
) -> impl Parser<'tokens, I, Spanned<Expr>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
    P: Parser<'tokens, I, Spanned<Expr>, extra::Err<ParseError<'tokens>>> + Clone + 'tokens,
    O: Parser<'tokens, I, Operator, extra::Err<ParseError<'tokens>>> + Clone + 'tokens,
{
    operand
        .clone()
        .foldl_with(op.then(operand).repeated(), |lhs, (op, rhs), extra| {
            let span = extra.span();
            if op.changes_binary_result_type() {
                return (
                    Expr::Binary {
                        operands: vec![lhs, rhs],
                        operators: vec![op],
                    },
                    span,
                );
            }

            let mut operands = Vec::new();
            let mut operators = Vec::new();
            append_same_type_operand(lhs, &mut operands, &mut operators);
            operators.push(op);
            append_same_type_operand(rhs, &mut operands, &mut operators);
            (
                Expr::Binary {
                    operands,
                    operators,
                },
                span,
            )
        })
        .boxed()
}

fn append_same_type_operand(
    expr: Spanned<Expr>,
    operands: &mut Vec<Spanned<Expr>>,
    operators: &mut Vec<Operator>,
) {
    let (expr, span) = expr;
    match expr {
        Expr::Binary {
            operands: nested_operands,
            operators: nested_operators,
        } if nested_operators
            .iter()
            .all(|op| !op.changes_binary_result_type()) =>
        {
            operands.extend(nested_operands);
            operators.extend(nested_operators);
        }
        other => operands.push((other, span)),
    }
}

pub fn expression_parser<'tokens, I>()
-> impl Parser<'tokens, I, Spanned<Expr>, extra::Err<ParseError<'tokens>>> + Clone
where
    I: ValueInput<'tokens, Token = TokenPack, Span = Span>,
{
    recursive(|expr| {
        let literal = select! {
            TokenPack::Data { kind, text } => (kind, text),
        }
        .map(|(kind, text)| {
            let kind = match kind {
                Token::IntLiteral => LiteralKind::Integer,
                Token::FloatLiteral => LiteralKind::Float,
                Token::StringLiteral => LiteralKind::String,
                Token::True | Token::False => LiteralKind::Boolean,
                Token::Null => LiteralKind::Null,
                _ => unreachable!("lexer emitted a non-literal as data"),
            };
            Expr::Literal { kind, text }
        })
        .map_with(|expr, extra| (expr, extra.span()));

        let path_expr = path()
            .map(Expr::Path)
            .map_with(|expr, extra| (expr, extra.span()));

        let list = expr
            .clone()
            .separated_by(operator(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>();

        let init_list = list
            .clone()
            .delimited_by(keyword(Token::LBrace), keyword(Token::RBrace));

        let parenthesized = expr
            .clone()
            .delimited_by(keyword(Token::LParen), keyword(Token::RParen))
            .map_with(|expr, extra| (Expr::Group(Box::new(expr)), extra.span()));

        let bare_init = init_list
            .clone()
            .map(|values| Expr::Init {
                target: None,
                values,
            })
            .map_with(|expr, extra| (expr, extra.span()));

        let typed_init = type_parser()
            .then(init_list.clone())
            .map(|(target, values)| Expr::Init {
                target: Some(target),
                values,
            })
            .map_with(|expr, extra| (expr, extra.span()));

        let array = list
            .clone()
            .delimited_by(keyword(Token::LeftBracket), keyword(Token::RightBracket))
            .map(Expr::Array)
            .map_with(|expr, extra| (expr, extra.span()));

        let atom = choice((
            literal,
            typed_init,
            path_expr,
            parenthesized,
            bare_init,
            array,
        ))
        .labelled("expression");

        let call = list
            .clone()
            .delimited_by(keyword(Token::LParen), keyword(Token::RParen))
            .map(Postfix::Call);

        let member = choice((
            operator(Token::Dot).to(false),
            operator(Token::Arrow).to(true),
        ))
        .then(spanned_ident())
        .map(|(access, name)| Postfix::Member(access, name));

        let index = expr
            .clone()
            .delimited_by(keyword(Token::LeftBracket), keyword(Token::RightBracket))
            .map(Postfix::Index);

        let postfix = atom.foldl_with(
            choice((call, member, index)).repeated(),
            |base, postfix, extra| {
                let expr = match postfix {
                    Postfix::Call(args) => Expr::Call {
                        callee: Box::new(base),
                        args,
                    },
                    Postfix::Member(indirect, (name, name_span)) => Expr::Member {
                        base: Box::new(base),
                        indirect,
                        name,
                        name_span,
                    },
                    Postfix::Index(index) => Expr::Binary {
                        operands: vec![base, index],
                        operators: vec![Operator::Index],
                    },
                };
                (expr, extra.span())
            },
        );

        let prefix = choice((
            operator(Token::Minus).to(Operator::Negate),
            operator(Token::LogicalNot).to(Operator::LogicalNot),
            operator(Token::BitNot).to(Operator::BitNot),
            operator(Token::AddressOf).to(Operator::AddressOf),
            operator(Token::Dereference).to(Operator::Dereference),
        ));

        let unary = prefix
            .repeated()
            .collect::<Vec<_>>()
            .then(postfix)
            .map_with(|(ops, value), extra| {
                let span = extra.span();
                ops.into_iter().rev().fold(value, |value, op| {
                    (
                        Expr::Unary {
                            op,
                            value: Box::new(value),
                        },
                        span,
                    )
                })
            });

        let product_op = choice((
            binary(Token::Multi, Operator::Multiply),
            binary(Token::Div, Operator::Divide),
            binary(Token::Mod, Operator::Remainder),
        ));
        let product = fold_binary(unary, product_op);

        let sum_op = choice((
            binary(Token::Plus, Operator::Add),
            binary(Token::Minus, Operator::Subtract),
        ));
        let sum = fold_binary(product, sum_op);

        let shift_op = choice((
            binary(Token::Shl, Operator::ShiftLeft),
            binary(Token::Shr, Operator::ShiftRight),
        ));
        let shift = fold_binary(sum, shift_op);

        let compare_op = choice((
            binary(Token::Equal, Operator::Equal),
            binary(Token::NotEqual, Operator::NotEqual),
            binary(Token::LAngle, Operator::Less),
            binary(Token::LessOrEqual, Operator::LessOrEqual),
            binary(Token::RAngle, Operator::Greater),
            binary(Token::GreaterOrEqual, Operator::GreaterOrEqual),
        ));
        let compare = fold_binary(shift, compare_op);
        let bit_and = fold_binary(compare, binary(Token::BitAnd, Operator::BitAnd));
        let bit_xor = fold_binary(bit_and, binary(Token::BitXor, Operator::BitXor));
        let bit_or = fold_binary(bit_xor, binary(Token::BitOr, Operator::BitOr));
        let logical_and = fold_binary(bit_or, binary(Token::LogicalAnd, Operator::LogicalAnd));
        let logical_or = fold_binary(logical_and, binary(Token::LogicalOr, Operator::LogicalOr));

        let pipe_target = path()
            .then(
                list.clone()
                    .delimited_by(keyword(Token::LParen), keyword(Token::RParen))
                    .or_not(),
            )
            .map_with(|target, extra| (target, extra.span()));

        logical_or.foldl_with(
            keyword(Token::Pipe).ignore_then(pipe_target).repeated(),
            |input, ((callee, explicit_args), callee_span), extra| {
                let mut args = match input {
                    (
                        Expr::Init {
                            target: None,
                            values,
                        },
                        _,
                    ) => values,
                    input => vec![input],
                };
                args.extend(explicit_args.unwrap_or_default());

                let callee = (Expr::Path(callee), callee_span);
                (
                    Expr::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    extra.span(),
                )
            },
        )
    })
    .boxed()
}
