//! Delimiter layout planning, separate from comment emission and token spelling.
use crate::{hints::Hints, render::kind};
use mlc_syntax::{
    lexer::{Lexed, token::Token},
    parser::out::TempModule,
};

const WIDTH: usize = 30;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Style {
    Block,
    Members,
    Vertical,
    Packed,
    Inline,
}

#[derive(Default, Clone)]
pub(super) struct Action {
    pub line: bool,
    pub indent_before: usize,
    pub indent_after: usize,
    pub tight_before: bool,
    pub tight_after: bool,
    pub block_end: bool,
}

struct Group {
    open: usize,
    close: usize,
    style: Style,
    commas: Vec<usize>,
    children: Vec<usize>,
    expanded: bool,
}

pub(super) fn plan(source: &str, lexed: &Lexed, module: &TempModule) -> Vec<Action> {
    use Token::*;
    let hints = Hints::collect(lexed, module);
    let mut groups: Vec<Group> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut actions = vec![Action::default(); lexed.tokens.len()];
    for (index, (token, _)) in lexed.tokens.iter().enumerate() {
        let token = kind(token);
        let style = match token {
            LBrace => Some(Style::Block),
            LParen | AttributeStart => Some(Style::Inline),
            LeftBracket => Some(Style::Packed),
            LAngle if hints.contains_key(&index) => Some(Style::Vertical),
            _ => None,
        };
        if let Some(default) = style {
            let id = groups.len();
            if let Some(parent) = stack.last() {
                groups[*parent].children.push(id);
            }
            groups.push(Group {
                open: index,
                close: index,
                style: hints.get(&index).copied().unwrap_or(default),
                commas: vec![],
                children: vec![],
                expanded: false,
            });
            stack.push(id);
        } else if token == Comma {
            if let Some(parent) = stack.last() {
                groups[*parent].commas.push(index);
            }
        } else {
            let count = match token {
                RBrace | RParen | RightBracket | AttributeEnd => 1,
                RAngle
                    if stack
                        .last()
                        .is_some_and(|id| kind(&lexed.tokens[groups[*id].open].0) == LAngle) =>
                {
                    1
                }
                Shr if stack
                    .last()
                    .is_some_and(|id| kind(&lexed.tokens[groups[*id].open].0) == LAngle) =>
                {
                    2
                }
                _ => 0,
            };
            for _ in 0..count {
                if let Some(id) = stack.pop() {
                    groups[id].close = index;
                    // Pipe argument packs have already been flattened into Call args.
                    if token == RBrace
                        && lexed
                            .tokens
                            .get(index + 1)
                            .is_some_and(|(token, _)| kind(token) == Pipe)
                    {
                        groups[id].style = Style::Packed;
                    }
                }
            }
        }
        if token == AttributeStart {
            actions[index].line = true;
        }
        if token == AttributeEnd && index + 1 < actions.len() {
            actions[index + 1].line = true;
        }
    }

    // Prefix sums make widths and comment-presence queries independent of nesting depth.
    let mut widths = vec![0usize];
    for (_, span) in &lexed.tokens {
        widths.push(widths.last().unwrap() + source[span.into_range()].chars().count());
    }
    for id in (0..groups.len()).rev() {
        let group = &groups[id];
        let (open, close, style) = (group.open, group.close, group.style);
        if close <= open {
            continue;
        }
        let start = lexed.tokens[open].1.end;
        let end = lexed.tokens[close].1.start;
        let first_comment = lexed
            .comments
            .partition_point(|comment| comment.span.end <= start);
        let comments = lexed.comments[first_comment..]
            .iter()
            .take_while(|comment| comment.span.start < end);
        let comment_break = comments.into_iter().any(|comment| {
            // A nested initializer manages its own comments; don't detach it
            // from a surrounding single-argument call merely because of them.
            if group.children.iter().any(|child| {
                let child = &groups[*child];
                comment.span.start >= lexed.tokens[child.open].1.end
                    && comment.span.end <= lexed.tokens[child.close].1.start
            }) {
                return false;
            }
            let gap = lexed
                .tokens
                .partition_point(|(_, span)| span.end <= comment.span.start);
            let previous_end = gap
                .checked_sub(1)
                .map_or(0, |index| lexed.tokens[index].1.end);
            comment.text.starts_with("//")
                || comment.text.contains(['\r', '\n'])
                || source[previous_end..comment.span.start].contains(['\r', '\n'])
        });
        let child_multiline = group.children.iter().any(|child| groups[*child].expanded);
        let items = item_ranges(group);
        // Keep a sole nested initializer attached to its call: add(One { ... }).
        let nested_single = style == Style::Vertical
            && kind(&lexed.tokens[open].0) == LParen
            && items.len() == 1
            && child_multiline;
        let expanded = match style {
            Style::Block | Style::Members => true,
            Style::Inline => false,
            _ => comment_break || (!nested_single && widths[close] - widths[open + 1] > WIDTH),
        };
        groups[id].expanded = expanded;
        actions[open].tight_after = style != Style::Block && !expanded;
        actions[close].tight_before = style != Style::Block;
        if kind(&lexed.tokens[open].0) == LAngle {
            actions[open].tight_before = true;
        }
        if style == Style::Block {
            actions[close].block_end = true;
        }
        if !expanded {
            continue;
        }
        actions[open].indent_after += 1;
        actions[close].indent_before += 1;
        actions[close].line = true;
        if open + 1 < close {
            actions[open + 1].line = true;
        }
        let mut line_width = 0;
        for (position, &(start, end)) in items.iter().enumerate() {
            let width = widths[end] - widths[start];
            // Include a separator's spelling, but never the formatter's added spaces.
            let separator = usize::from(end < close);
            let width = width + separator;
            if position > 0 && (style != Style::Packed || line_width + width > WIDTH) {
                actions[start].line = true;
                line_width = 0;
            }
            line_width += width;
            if groups[id].children.iter().any(|child| {
                groups[*child].expanded && groups[*child].open >= start && groups[*child].open < end
            }) {
                line_width = WIDTH;
            }
        }
    }
    actions
}

fn item_ranges(group: &Group) -> Vec<(usize, usize)> {
    let mut start = group.open + 1;
    let mut items = vec![];
    for &comma in &group.commas {
        if start < comma {
            items.push((start, comma));
        }
        start = comma + 1;
    }
    if start < group.close {
        items.push((start, group.close));
    }
    items
}
