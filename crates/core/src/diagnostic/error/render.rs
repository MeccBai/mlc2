use colored::{Color, Colorize};
use std::{fmt::Write, ops::Range};
#[cfg(test)]
mod tests;

fn boundary(source: &str, offset: usize) -> usize {
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(super) fn diagnostic(file: &str, source: &str, span: Range<usize>, message: &str) -> String {
    diagnostic_with_color(file, source, span, message, "error", Color::Red)
}

pub(super) fn warning(file: &str, source: &str, span: Range<usize>, message: &str) -> String {
    diagnostic_with_color(file, source, span, message, "warning", Color::Yellow)
}

fn diagnostic_with_color(
    file: &str,
    source: &str,
    span: Range<usize>,
    message: &str,
    label: &str,
    color: Color,
) -> String {
    let start = boundary(source, span.start);
    let line = source[..start].bytes().filter(|b| *b == b'\n').count() + 1;
    let line_start = source[..start].rfind('\n').map_or(0, |n| n + 1);
    let column = source[line_start..start].chars().count() + 1;
    // Canonical Windows paths are useful internally, not as a display prefix.
    let file = if let Some(file) = file.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{file}")
    } else {
        file.strip_prefix(r"\\?\").unwrap_or(file).to_owned()
    };
    format!(
        "{}: {message}\n  --> {file}:{line}:{column}\n{}",
        label.color(color).bold(),
        context_with_color(source, 2, 2, span, color)
    )
}

pub(super) fn context(source: &str, before: usize, after: usize, span: Range<usize>) -> String {
    context_with_color(source, before, after, span, Color::Red)
}

fn context_with_color(
    source: &str,
    before: usize,
    after: usize,
    span: Range<usize>,
    color: Color,
) -> String {
    let start = boundary(source, span.start);
    let end = boundary(source, span.end).max(start);
    let lines: Vec<_> = source.split('\n').collect();
    let first = source[..start].bytes().filter(|b| *b == b'\n').count();
    let last_offset = if end > start {
        boundary(source, end - 1)
    } else {
        start
    };
    let last = source[..last_offset]
        .bytes()
        .filter(|b| *b == b'\n')
        .count();
    let mut result = String::new();
    let mut offset = 0;
    for (index, raw) in lines.iter().enumerate() {
        let text = raw.strip_suffix('\r').unwrap_or(raw);
        if index >= first.saturating_sub(before) && index <= last.saturating_add(after) {
            let lo = start.saturating_sub(offset).min(text.len());
            let hi = end.saturating_sub(offset).min(text.len());
            let marked = index >= first && index <= last;
            if marked && hi > lo {
                writeln!(
                    result,
                    "{:4} | {}{}{}",
                    index + 1,
                    &text[..lo],
                    text[lo..hi].color(color).bold(),
                    &text[hi..]
                )
                .unwrap();
            } else {
                writeln!(result, "{:4} | {text}", index + 1).unwrap();
            }
            if marked {
                let padding = text[..lo]
                    .chars()
                    .map(|c| if c == '\t' { '\t' } else { ' ' })
                    .collect::<String>();
                let width = text[lo..hi].chars().count().max(1);
                writeln!(
                    result,
                    "     | {}{}",
                    padding,
                    "^".repeat(width).color(color).bold()
                )
                .unwrap();
            }
        }
        offset += raw.len() + 1;
    }
    result
}
