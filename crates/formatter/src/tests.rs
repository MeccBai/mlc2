use super::*;
mod layout;
mod separation;

fn stable(source: &str) -> String {
    let output = format(source).unwrap_or_else(|error| panic!("{error}\n{source}"));
    assert_eq!(
        format(&output).unwrap(),
        output,
        "formatting must be idempotent"
    );
    let lexed = lexer::tokenize(&output).unwrap();
    let original = lexer::tokenize(source).unwrap();
    let comment_gaps = |lexed: &lexer::Lexed| {
        lexed
            .comments
            .iter()
            .map(|comment| {
                lexed
                    .tokens
                    .partition_point(|(_, span)| span.end <= comment.span.start)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        comment_gaps(&original),
        comment_gaps(&lexed),
        "comments must stay between the same tokens"
    );
    let (module, errors) = parser::parse(&lexed.tokens, output.len());
    assert!(module.is_some() && errors.is_empty(), "{errors:?}");
    output
}

#[test]
fn fixed_indentation_and_spacing() {
    assert_eq!(
        stable("func main(){var a:i32=1;return;}"),
        "func main() {\n    var a: i32 = 1;\n    return;\n}\n"
    );
}

#[test]
fn comments_keep_their_token_gaps() {
    let source = "// 文件头\r\nfunc main(){// entry\r\nvar a=1; // trailing\r\n/* before */ a = a /* middle */ + 1;\r\n// empty tail\r\n}\r\n// EOF";
    let output = stable(source);
    assert!(output.contains("var a = 1; // trailing\n"));
    assert!(output.contains("a /* middle */ + 1"));
    assert!(output.ends_with("// EOF\n"));
    assert_eq!(lexer::tokenize(source).unwrap().comments.len(), 7);
}

#[test]
fn empty_files_blocks_and_adjacent_comments() {
    assert_eq!(stable(""), "");
    assert_eq!(stable("// alone"), "// alone\n");
    stable("/* one */ /* two */\nfunc main(){/* empty */}");
    stable("func main(){var x=1;// a\n// b\nx=x+1;}");
    stable("// keep trailing spaces   \n/* block */");
    stable("func main(){var x=1 .value;}");
}

#[test]
fn refuses_invalid_syntax_but_does_not_resolve_names() {
    assert!(matches!(
        format("func main(){var a;}"),
        Err(FormatError::Syntax(_))
    ));
    assert!(matches!(format("`"), Err(FormatError::Lexical(_))));
    stable("import unavailable::module; func main(){var x:Unknown = missing();}");
}

#[test]
fn preserves_macro_generic_and_expression_tokens() {
    stable(
        "func<T> identity(x:T)->T{return x;} func main(){var x=identity<i32>(1);print!(\"{}\",x);if(x<2){x=-x;}else if(x>4){x=x-1;}}",
    );
    stable("unit P{pub value:i32;};func main(){var p=P{1};var a:[i32:2]=[1,2];a[0]=p.value;}");
}

#[test]
fn repository_sources_are_idempotent() {
    fn visit(path: &std::path::Path) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path);
            } else if path.extension().is_some_and(|extension| extension == "m2") {
                stable(&std::fs::read_to_string(path).unwrap());
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    visit(&root.join("lib"));
    visit(&root.join("crates/examples/sources"));
}
