use super::stable;

#[test]
fn declarations_have_exact_blank_line_counts_without_eof_padding() {
    let output = stable("unit A{};union B[i32];generic C{};func f(){}func g(){}");
    assert!(output.contains("};\n\nunion B"), "{output}");
    assert!(output.contains("];\n\ngeneric C"), "{output}");
    assert!(output.contains("};\n\nfunc f"), "{output}");
    assert!(output.contains("}\n\n\nfunc g"), "{output}");
    assert!(output.ends_with("}\n"));
    assert!(!output.ends_with("}\n\n"));
}

#[test]
fn consecutive_imports_and_export_aliases_are_a_single_group() {
    let output = stable("export import a;import b;export using C=D;func f(){}");
    assert!(
        output.starts_with("export import a;\nimport b;\nexport using C = D;\n\n\nfunc f"),
        "{output}"
    );
}

#[test]
fn trailing_comments_stay_attached_and_leading_comments_follow_the_gap() {
    let output = stable("func a(){} // trailing\n// describes b\n#[test]# func b(){}");
    assert!(
        output.contains("} // trailing\n\n\n// describes b\n#[test]#\nfunc b"),
        "{output}"
    );
    let output = stable("unit A{}; /* trailing */\n/* describes b */func b(){}");
    assert!(
        output.contains("}; /* trailing */\n\n/* describes b */\nfunc b"),
        "{output}"
    );
}
