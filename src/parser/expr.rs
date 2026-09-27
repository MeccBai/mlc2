use chumsky::{input::ValueInput, prelude::*};

use crate::lexer::{TokenPack, token::Token};

use super::{
    BinaryOp, Expr, LiteralKind, MemberAccess, ParseError, Spanned, UnaryOp,
    out::Span,
    split::{keyword, operator, path},
};

enum Postfix {
    Call(Vec<Spanned<Expr>>),
    Member(MemberAccess, String),
    Init(Vec<Spanned<Expr>>),
}

fn binary<'tokens, I>(
    token: Token,
    op: BinaryOp,
) -> impl Parser<'tokens, I, BinaryOp, extra::Err<ParseError<'tokens>>> + Clone
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
    O: Parser<'tokens, I, BinaryOp, extra::Err<ParseError<'tokens>>> + Clone + 'tokens,
{
    operand
        .clone()
        .foldl_with(op.then(operand).repeated(), |lhs, (op, rhs), extra| {
            (
                Expr::Binary {
                    lhs: Box::new(lhs),
                    op,
                    rhs: Box::new(rhs),
                },
                extra.span(),
            )
        })
        .boxed()
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
            .map_with(|(expr, _), extra| (expr, extra.span()));

        let bare_init = init_list
            .clone()
            .map(|values| Expr::Init {
                target: None,
                values,
            })
            .map_with(|expr, extra| (expr, extra.span()));

        let array = list
            .clone()
            .delimited_by(keyword(Token::LeftBracket), keyword(Token::RightBracket))
            .map(Expr::Array)
            .map_with(|expr, extra| (expr, extra.span()));

        let atom =
            choice((literal, path_expr, parenthesized, bare_init, array)).labelled("expression");

        let call = list
            .clone()
            .delimited_by(keyword(Token::LParen), keyword(Token::RParen))
            .map(Postfix::Call);

        let member = choice((
            operator(Token::Dot).to(MemberAccess::Dot),
            operator(Token::Arrow).to(MemberAccess::Arrow),
        ))
        .then(super::split::ident())
        .map(|(access, name)| Postfix::Member(access, name));

        let postfix_init = init_list.map(Postfix::Init);

        let postfix = atom.foldl_with(
            choice((call, member, postfix_init)).repeated(),
            |base, postfix, extra| {
                let expr = match postfix {
                    Postfix::Call(args) => Expr::Call {
                        callee: Box::new(base),
                        args,
                    },
                    Postfix::Member(access, name) => Expr::Member {
                        base: Box::new(base),
                        access,
                        name,
                    },
                    Postfix::Init(values) => Expr::Init {
                        target: Some(Box::new(base)),
                        values,
                    },
                };
                (expr, extra.span())
            },
        );

        let prefix = choice((
            operator(Token::Minus).to(UnaryOp::Negate),
            operator(Token::LogicalNot).to(UnaryOp::LogicalNot),
            operator(Token::BitNot).to(UnaryOp::BitNot),
            operator(Token::AddressOf).to(UnaryOp::AddressOf),
            operator(Token::Dereference).to(UnaryOp::Dereference),
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
            binary(Token::Multi, BinaryOp::Multiply),
            binary(Token::Div, BinaryOp::Divide),
            binary(Token::Mod, BinaryOp::Remainder),
        ));
        let product = fold_binary(unary, product_op);

        let sum_op = choice((
            binary(Token::Plus, BinaryOp::Add),
            binary(Token::Minus, BinaryOp::Subtract),
        ));
        let sum = fold_binary(product, sum_op);

        let shift_op = choice((
            binary(Token::Shl, BinaryOp::ShiftLeft),
            binary(Token::Shr, BinaryOp::ShiftRight),
        ));
        let shift = fold_binary(sum, shift_op);

        let compare_op = choice((
            binary(Token::Equal, BinaryOp::Equal),
            binary(Token::NotEqual, BinaryOp::NotEqual),
            binary(Token::LAngle, BinaryOp::Less),
            binary(Token::LessOrEqual, BinaryOp::LessOrEqual),
            binary(Token::RAngle, BinaryOp::Greater),
            binary(Token::GreaterOrEqual, BinaryOp::GreaterOrEqual),
        ));
        let compare = fold_binary(shift, compare_op);
        let bit_and = fold_binary(compare, binary(Token::BitAnd, BinaryOp::BitAnd));
        let bit_xor = fold_binary(bit_and, binary(Token::BitXor, BinaryOp::BitXor));
        let bit_or = fold_binary(bit_xor, binary(Token::BitOr, BinaryOp::BitOr));
        let logical_and = fold_binary(bit_or, binary(Token::LogicalAnd, BinaryOp::LogicalAnd));
        let logical_or = fold_binary(logical_and, binary(Token::LogicalOr, BinaryOp::LogicalOr));

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
