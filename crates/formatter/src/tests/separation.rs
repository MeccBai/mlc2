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
fn imports_and_using_have_separate_group_gaps() {
    let output = stable("export import a;import b;export using C=D;func f(){}");
    assert!(
        output.starts_with("export import a;\nimport b;\n\n\nexport using C = D;\n\nfunc f"),
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

#[test]
fn import_and_using_groups_count_declarations() {
    for (source, expected) in [
        ("import a;func f(){}", "import a;\n\nfunc f"),
        (
            "import a;import b;func f(){}",
            "import a;\nimport b;\n\n\nfunc f",
        ),
        ("using A=B;func f(){}", "using A = B;\n\nfunc f"),
        (
            "using A=B;using C=D;func f(){}",
            "using A = B;\nusing C = D;\n\n\nfunc f",
        ),
        (
            "import a;using A=B;using C=D;func f(){}",
            "import a;\n\nusing A = B;\nusing C = D;\n\n\nfunc f",
        ),
    ] {
        assert!(stable(source).starts_with(expected), "{source}");
    }
    assert_eq!(stable("import a;"), "import a;\n");
    assert_eq!(stable("using A=B;"), "using A = B;\n");
}

#[test]
fn imports_sort_by_depth_then_length_with_stable_ties() {
    let output = stable(
        "import a::b::c;import longer;export import z;import zz::b;import x::y;import a::b;func f(){}",
    );
    assert!(output.starts_with("export import z;\nimport longer;\nimport x::y;\nimport a::b;\nimport zz::b;\nimport a::b::c;\n\n\nfunc f"), "{output}");
    assert!(
        stable("import zz;using A=B;import a;")
            .starts_with("import zz;\n\nusing A = B;\n\nimport a;")
    );
}

#[test]
fn import_comments_move_with_their_declarations() {
    let output = stable(
        "// header\r\nimport longer; // long\r\n// short\r\nexport import a; // tiny\r\nfunc f(){}",
    );
    assert!(
        output.starts_with(
            "// header\n// short\nexport import a; // tiny\nimport longer; // long\n\n\nfunc f"
        ),
        "{output}"
    );
    let output = stable("import longer /* inside */; /* tail */\n/* short */ import a; // final\n");
    assert!(
        output.contains("/* short */ import a; // final\nimport longer /* inside */ ; /* tail */"),
        "{output}"
    );
}
