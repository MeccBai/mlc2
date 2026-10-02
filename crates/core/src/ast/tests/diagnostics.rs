use super::*;
use crate::diagnostic::error::{IllegalUseError, ResolveError};

#[test]
fn unknown_type_reports_original_source_location() {
    assert_error(
        "func main() { var a:Missing = 1; }",
        "Missing",
        CompileError::Resolve(ResolveError::UnknownType),
    );
}

#[test]
fn unknown_variable_stops_following_statements_without_panicking() {
    assert_error(
        "func main() { var a = missing; var b:i32 = true; var c = also_missing; }",
        "missing",
        CompileError::Resolve(ResolveError::UnknownVariable),
    );
}

#[test]
fn wrong_condition_stops_branch_errors() {
    assert_error(
        "func main() { if (123) { var a = missing; } else { var b = also_missing; } }",
        "123",
        CompileError::IllegalUse(IllegalUseError::ExpressionMustConditional),
    );
}

#[test]
fn match_type_error_precedes_branch_body_errors() {
    assert_error(
        "func main() { match (1) { true => { var a = missing; } } }",
        "true",
        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
            expected: "i32".into(),
            found: "bool".into(),
        }),
    );
}

#[test]
fn invalid_for_bound_preserves_span() {
    assert_error(
        "func main() { for i in [0, true] { var a = missing; } }",
        "true",
        CompileError::IllegalUse(IllegalUseError::ForBoundMustBeInteger),
    );
}

#[test]
fn multibyte_comments_do_not_corrupt_error_offsets() {
    assert_error(
        "// 中文注释\nfunc main() { var a = missing; }",
        "missing",
        CompileError::Resolve(ResolveError::UnknownVariable),
    );
}

#[test]
fn malformed_text_is_rejected_before_ast_construction() {
    for source in [
        "func main() { var a = ; }",
        "func main( {",
        "unit Point { x:; };",
        "func<T wrong() {}",
        "func main() { match (1) { 1 {} } }",
        "func main() { var p = @mut; }",
        "func main() { var a = P{1; }",
    ] {
        invalid_syntax(source);
    }
}

#[test]
fn invalid_character_produces_lexical_error_without_panic() {
    assert!(matches!(pipeline("func main() { ` }"), Outcome::LexError));
}

#[test]
fn truncating_valid_source_never_panics_in_the_pipeline() {
    // A deterministic recovery corpus: cuts at every UTF-8 boundary, including
    // inside names, expressions, generic arguments and unfinished scopes.
    for source in [
        "// 中文\nfunc main() { var a:i32 = 1 + (2 * 3); }",
        "unit Box<T> { value:$T; }; func main() { var a = 1; var b = Box<i32>{@a}; }",
        "func<T> value(a:T) -> T { return a; } func main() { value<i32>(1); }",
        "func main() { if (true) { for i in [0,2] { match (i) { 1 => {}, _ => {} } } } }",
        "func main() { var a = [1,2]; var p = @mut a; var value = a[0]; }",
    ] {
        for end in source
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(source.len()))
        {
            // Either an error or a valid prefix is acceptable; a panic is not.
            pipeline(&source[..end]);
        }
    }
}
