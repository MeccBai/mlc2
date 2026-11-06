//! Blank lines between top-level declarations, never inside a statement block.
use mlc_syntax::{
    lexer::Lexed,
    parser::out::{TempGlobalStmt, TempModule},
};

pub(super) fn gaps(lexed: &Lexed, module: &TempModule) -> Vec<usize> {
    let mut gaps = vec![0; lexed.tokens.len() + 1];
    let mut group_count = 0;
    for (position, pair) in module.windows(2).enumerate() {
        let (previous, _) = &pair[0];
        let (next, span) = &pair[1];
        let category = directive(previous);
        if category.is_some() {
            group_count = if position > 0 && directive(&module[position - 1].0) == category {
                group_count + 1
            } else {
                1
            };
        } else {
            group_count = 0;
        }
        let blank_lines = match previous {
            TempGlobalStmt::Func(_)
            | TempGlobalStmt::Interface(_)
            | TempGlobalStmt::MacroFunc(_) => 2,
            TempGlobalStmt::Unit(_) | TempGlobalStmt::Generic(_) => 1,
            _ if category.is_some() && category != directive(next) => group_count.min(2),
            _ => 0,
        };
        let index = lexed
            .tokens
            .partition_point(|(_, token_span)| token_span.start < span.start);
        gaps[index] = blank_lines;
    }
    gaps
}

fn directive(item: &TempGlobalStmt) -> Option<bool> {
    match item {
        TempGlobalStmt::Import(_) => Some(false),
        TempGlobalStmt::Using(_) => Some(true),
        _ => None,
    }
}
