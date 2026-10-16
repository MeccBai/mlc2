use super::Span;

/// One-based source coordinates; columns count UTF-8 bytes, not display cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

/// Built once from the original source; token and AST spans remain byte offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTable {
    line_starts: Vec<usize>,
    source_len: usize,
}

impl LineTable {
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = vec![0];
        let mut offset = 0;
        while offset < bytes.len() {
            match bytes[offset] {
                b'\r' => {
                    offset += 1;
                    if bytes.get(offset) == Some(&b'\n') { offset += 1; }
                    line_starts.push(offset);
                }
                b'\n' => { offset += 1; line_starts.push(offset); }
                _ => offset += 1,
            }
        }
        Self { line_starts, source_len: bytes.len() }
    }

    pub fn line_starts(&self) -> &[usize] { &self.line_starts }

    /// EOF is a valid position; offsets outside this source are not.
    pub fn position(&self, offset: usize) -> Option<SourcePosition> {
        if offset > self.source_len { return None; }
        let index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        Some(SourcePosition { line: index + 1, column: offset - self.line_starts[index] + 1 })
    }

    pub fn span_start(&self, span: Span) -> Option<SourcePosition> {
        if span.start > span.end || span.end > self.source_len { return None; }
        self.position(span.start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_endings_utf8_and_eof_preserve_original_offsets() {
        let source = "中\r\nabc\nx\ry\n";
        let table = LineTable::new(source);
        assert_eq!(table.line_starts(), &[0, 5, 9, 11, 13]);
        assert_eq!(table.position(6), Some(SourcePosition { line: 2, column: 2 }));
        assert_eq!(table.position(13), Some(SourcePosition { line: 5, column: 1 }));
        assert!(table.position(14).is_none());
        assert!(table.span_start((2..14).into()).is_none());
    }

    #[test]
    fn empty_and_single_line_sources_have_a_first_line() {
        assert_eq!(LineTable::new("").position(0), Some(SourcePosition { line: 1, column: 1 }));
        assert_eq!(LineTable::new("abc").position(3), Some(SourcePosition { line: 1, column: 4 }));
    }
}
