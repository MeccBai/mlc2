//! Blank lines between top-level declarations, never inside a statement block.
use mlc_syntax::{
    lexer::Lexed,
    parser::out::{TempGlobalStmt, TempModule, TempVisibility},
};

pub(super) fn gaps(lexed: &Lexed, module: &TempModule) -> Vec<usize> {
    let mut gaps = vec![0; lexed.tokens.len() + 1];
    for pair in module.windows(2) {
        let (previous, _) = &pair[0];
        let (next, span) = &pair[1];
        let blank_lines = match previous {
            TempGlobalStmt::Func(_)
            | TempGlobalStmt::Interface(_)
            | TempGlobalStmt::MacroFunc(_) => 2,
            TempGlobalStmt::Unit(_) | TempGlobalStmt::Generic(_) => 1,
            _ if directive(previous) && !directive(next) => 2,
            _ => 0,
        };
        let index = lexed
            .tokens
            .partition_point(|(_, token_span)| token_span.start < span.start);
        gaps[index] = blank_lines;
    }
    gaps
}

fn directive(item: &TempGlobalStmt) -> bool {
    match item {
        TempGlobalStmt::Import(_) => true,
        TempGlobalStmt::Using(using) => matches!(
            using.visibility,
            TempVisibility::Export | TempVisibility::Api
        ),
        _ => false,
    }
}
