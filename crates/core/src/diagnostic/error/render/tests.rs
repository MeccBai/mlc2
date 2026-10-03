use super::*;
use crate::diagnostic::error::{CompileError, ErrorHandle, IllegalUseError};

#[test]
fn human_message_file_and_two_context_lines() {
    let source = "one\ntwo\nreturn 1;\nfour\nfive\nsix";
    let mut errors = ErrorHandle::new(r"\\?\C:\demo\main.m2".into());
    errors.submit_error(
        CompileError::IllegalUse(IllegalUseError::UnexpectedReturnValue),
        (15..16).into(),
    );
    let text = errors.render(source);
    assert!(text.contains("Cannot return a value"));
    assert!(text.trim_end().ends_with("1 error generated."));
    assert!(text.contains(r"C:\demo\main.m2:3:8"));
    assert!(!text.contains(r"\\?\"));
    assert!(!text.contains("ErrorHandle"));
    assert!(!text.contains("UnexpectedReturnValue"));
    assert!(text.contains("   1 | one"));
    assert!(text.contains("   5 | five"));
    assert!(!text.contains("six"));
    assert!(text.contains('^'));
}

#[test]
fn error_count_matches_unique_submitted_diagnostics() {
    let mut errors = ErrorHandle::new("main.m2".into());
    let first = CompileError::IllegalUse(IllegalUseError::UnexpectedReturnValue);
    errors.submit_error(first.clone(), (0..1).into());
    errors.submit_error(first, (0..1).into());
    errors.submit_error(
        CompileError::IllegalUse(IllegalUseError::ReturnValueRequired),
        (2..3).into(),
    );
    let text = errors.render("abc");
    assert!(text.ends_with("2 errors generated."));
}

#[test]
fn multiline_crlf_unicode_empty_and_invalid_spans_are_safe() {
    let source = "第一行\r\n第二行\r\n第三行\r\n";
    let text = context(source, 2, 2, 3..24);
    assert!(text.contains("第一行"));
    assert!(text.contains("第三行"));
    assert_eq!(text.matches('^').count(), 5);
    assert!(!text.contains('\r'));
    for span in [0..0, 1..2, 999..1000, 12..3] {
        assert!(!context(source, 2, 2, span).is_empty());
    }
    assert!(context("", 2, 2, 0..0).contains('^'));
    assert!(context("x\n", 2, 2, 2..2).contains("   2 |"));
}
