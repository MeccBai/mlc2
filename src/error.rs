use crate::lexer::TokenError;
use crate::parser::ParseError;
use colored::Colorize;
use std::ops::Range;

pub struct ErrorHandle {
    pub file: String,
}

impl ErrorHandle {
    pub fn new(file: String) -> Self {
        Self { file }
    }

    pub fn get_context(
        &self,
        source: &str,
        pre_lines: usize,
        after_lines: usize,
        span: Range<usize>,
    ) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let error_line = source[..span.start].matches('\n').count();
        let start = error_line.saturating_sub(pre_lines);
        let end = (error_line + after_lines + 1).min(lines.len());
        let mut result = String::new();
        let line_start = source[..span.start].rfind('\n').map(|x| x + 1).unwrap_or(0);
        let local_start = span.start - line_start;
        let local_end = span.end - line_start;
        for i in start..end {
            let mut line = lines[i].to_string();
            if i == error_line {
                let before = &line[..local_start];
                let error = &line[local_start..local_end];
                let after = &line[local_end..];

                line = format!("{}{}{}", before, error.red().bold().underline(), after);
            }
            result.push_str(&format!("{:4} | {}\n", i + 1, line));
        }
        result
    }

    pub fn token_error(&self, token_error: TokenError, source: &str) {
        let message = format!(
            "{} : {} , {} : `{}` at position {};",
            "Error in file".red().bold(),
            format!("\"{}\"", self.file.green().bold()),
            "Type".red().bold(),
            token_error.context.yellow().bold(),
            token_error.span.start
        );
        eprintln!("{}", message);
        eprintln!("{}", self.get_context(source, 2, 2, token_error.span));
    }

    pub fn parse_error(&self, parse_error: &ParseError<'_>, source: &str) {
        let span = parse_error.span().into_range();
        eprintln!(
            "{}: {} at {}..{}",
            "Syntax error".red().bold(),
            parse_error.reason(),
            span.start,
            span.end,
        );
        eprintln!("{}", self.get_context(source, 2, 2, span));
    }
}
