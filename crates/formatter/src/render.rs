use mlc_syntax::lexer::{Lexed, TokenPack, token::Token};

#[derive(Default)]
struct Writer {
    output: String,
    indent: usize,
    pending_line: bool,
    pending_space: bool,
}

impl Writer {
    fn blank_lines(&mut self, count: usize) {
        self.newline();
        if !self.output.is_empty() {
            let existing = self.output.chars().rev().take_while(|c| *c == '\n').count();
            for _ in existing..count + 1 {
                self.output.push('\n');
            }
        }
    }

    fn newline(&mut self) {
        self.pending_space = false;
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.pending_line = false;
    }

    fn space(&mut self) {
        if !self.output.is_empty() && !self.output.ends_with([' ', '\n']) {
            self.pending_space = true;
        }
    }

    fn text(&mut self, text: &str) {
        if self.pending_space && !self.output.ends_with([' ', '\n']) {
            self.output.push(' ');
        }
        self.pending_space = false;
        if self.output.is_empty() || self.output.ends_with('\n') {
            for _ in 0..self.indent {
                self.output.push_str("    ");
            }
        }
        self.output.push_str(text);
    }
}

pub(super) fn kind(token: &TokenPack) -> Token {
    match token {
        TokenPack::KeyWord(token) | TokenPack::Operator(token) => token.clone(),
        TokenPack::Ident(_) => Token::Ident,
        TokenPack::Data { kind, .. } => kind.clone(),
    }
}

fn has_line(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

fn space_before(previous: Option<&Token>, current: &Token) -> bool {
    use Token::*;
    let Some(previous) = previous else {
        return false;
    };
    // A numeric literal followed by '.' must not become a different float token.
    if *current == Dot && matches!(previous, IntLiteral | FloatLiteral) {
        return true;
    }
    if matches!(
        current,
        Semicolon | Comma | Colon | ColonColon | Dot | RParen | RightBracket | AttributeEnd
    ) || matches!(
        previous,
        LParen
            | LeftBracket
            | AttributeStart
            | ColonColon
            | Dot
            | AddressOf
            | Dereference
            | BitNot
            | LogicalNot
    ) {
        return false;
    }
    if *current == LogicalNot && matches!(previous, Ident | RAngle) {
        return false;
    }
    if *current == LParen {
        return matches!(previous, If | While | Match | For);
    }
    if *current == LeftBracket {
        return matches!(previous, Assign | In | Return | Colon | Comma);
    }
    true
}

pub(super) fn render(
    source: &str,
    lexed: &Lexed,
    module: &mlc_syntax::parser::out::TempModule,
) -> String {
    use Token::*;
    let mut writer = Writer::default();
    let mut comments = lexed.comments.iter().peekable();
    let mut previous = None;
    let mut source_end = 0;
    let actions = crate::layout::plan(source, lexed, module);
    let gaps = crate::separation::gaps(lexed, module);

    // EOF is an explicit final gap, including comment-only files.
    for index in 0..=lexed.tokens.len() {
        let token = lexed.tokens.get(index);
        let start = token.map_or(source.len(), |(_, span)| span.start);
        let mut gap = gaps[index];
        while let Some(comment) = comments.next_if(|comment| comment.span.start < start) {
            let standalone = has_line(&source[source_end..comment.span.start]);
            if standalone {
                if gap > 0 {
                    writer.blank_lines(gap);
                    gap = 0;
                } else {
                    writer.newline();
                }
            } else {
                writer.space();
            }
            writer.text(&comment.text);
            source_end = comment.span.end;
            let next_start = comments
                .peek()
                .map_or(start, |next| next.span.start.min(start));
            if comment.text.starts_with("//")
                || standalone
                || has_line(&source[source_end..next_start])
            {
                writer.newline();
            } else {
                writer.space();
            }
        }
        let Some((packed, span)) = token else {
            break;
        };
        if gap > 0 {
            writer.blank_lines(gap);
        }
        let current = kind(packed);
        let action = &actions[index];
        let continuation = matches!(
            current,
            Semicolon | Comma | RParen | RightBracket | Dot | Arrow | Else | Pipe
        );
        writer.indent = writer.indent.saturating_sub(action.indent_before);
        if action.line {
            writer.newline();
        } else if writer.pending_line && !continuation {
            writer.newline();
        }
        let tight_after_previous = index > 0 && actions[index - 1].tight_after;
        // Separate original '>' tokens: merging them would produce a shift token.
        if matches!(previous, Some(RAngle | Shr)) && matches!(current, RAngle | Shr) {
            writer.space();
        }
        if !action.tight_before
            && !tight_after_previous
            && space_before(previous.as_ref(), &current)
        {
            writer.space();
        }
        writer.text(&source[span.into_range()]);
        writer.pending_line = action.block_end || matches!(current, Semicolon | AttributeEnd);
        writer.indent += action.indent_after;
        source_end = span.end;
        previous = Some(current);
    }
    writer.newline();
    writer.output
}
