use super::*;

#[test]
fn listing_contains_each_unique_example() {
    let listing = render("list").unwrap();
    let mut names = std::collections::HashSet::new();
    for example in EXAMPLES {
        assert!(names.insert(example.name));
        assert!(listing.lines().any(|line| line.starts_with(example.name)));
        assert_eq!(render(example.name).unwrap(), example.source);
    }
}

#[test]
fn unknown_example_reports_how_to_list() {
    assert!(render("missing").unwrap_err().contains("mlc example list"));
}

#[test]
fn all_sources_parse() {
    for example in EXAMPLES {
        let tokens = mlc_syntax::lexer::tokenize(example.source).unwrap();
        let (module, errors) = mlc_syntax::parser::parse(&tokens.tokens, example.source.len());
        assert!(errors.is_empty(), "{}: {errors:?}", example.name);
        assert!(module.is_some());
    }
}
