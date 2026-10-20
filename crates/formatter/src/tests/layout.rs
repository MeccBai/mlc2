use super::stable;

#[test]
fn signature_threshold_ignores_spaces_and_outer_delimiters() {
    let name = "x".repeat(26); // name + ':i32' = exactly 30.
    let output = stable(&format!("func f({name} : i32){{}}"));
    assert!(output.contains(&format!("f({name}: i32) {{")), "{output}");
    let name = "x".repeat(27);
    let output = stable(&format!("func f({name}:i32){{}}"));
    assert!(
        output.contains(&format!("f(\n    {name}: i32\n) {{")),
        "{output}"
    );
}

#[test]
fn function_parameters_and_call_arguments_expand_one_per_line() {
    let output = stable(
        "func f(first_argumentx:i32,second_argumentx:i32){g(first_argumentx,second_argumentx);}",
    );
    assert!(
        output.contains("f(\n    first_argumentx: i32,\n    second_argumentx: i32\n) {"),
        "{output}"
    );
    assert!(
        output.contains("g(\n        first_argumentx,\n        second_argumentx\n    );"),
        "{output}"
    );
}

#[test]
fn generics_use_ast_roles_without_changing_comparisons() {
    let source = "func<FirstLongParameter,SecondLongParameter> f(x:i32){if(x<3){x=x>>1;}} unit<FirstLongParameter,SecondLongParameter> P{a:FirstLongParameter;b:SecondLongParameter;};";
    let output = stable(source);
    assert!(
        output.contains("func<\n    FirstLongParameter,\n    SecondLongParameter\n> f(x: i32) {"),
        "{output}"
    );
    assert!(output.contains("if (x < 3) {"), "{output}");
    assert!(output.contains("x >> 1"), "{output}");
    assert!(
        output.contains("unit<\n    FirstLongParameter,\n    SecondLongParameter\n> P {"),
        "{output}"
    );
}

#[test]
fn short_initializers_and_arrays_remain_inline() {
    let output = stable("func main(){var p=One{1,2,3};var a=[1,2,3];{1,2,3}|>one;}");
    assert!(output.contains("One {1, 2, 3};"), "{output}");
    assert!(output.contains("[1, 2, 3]"), "{output}");
    assert!(output.contains("{1, 2, 3} |> one;"), "{output}");
}

#[test]
fn initializers_pack_complete_elements_and_keep_nested_call_attached() {
    let output = stable(
        "func main(){add(One{aaaaaaaaaa,bbbbbbbbbb,cccccccccc,dddddddddd});var a=[aaaaaaaaaa,bbbbbbbbbb,cccccccccc,dddddddddd];{aaaaaaaaaa,bbbbbbbbbb,cccccccccc,dddddddddd}|>one;}",
    );
    assert!(
        output.contains(
            "add(One {\n        aaaaaaaaaa, bbbbbbbbbb,\n        cccccccccc, dddddddddd\n    });"
        ),
        "{output}"
    );
    assert!(
        output.contains(
            "var a = [\n        aaaaaaaaaa, bbbbbbbbbb,\n        cccccccccc, dddddddddd\n    ];"
        ),
        "{output}"
    );
    assert!(output.contains("\n    {\n        aaaaaaaaaa, bbbbbbbbbb,\n        cccccccccc, dddddddddd\n    } |> one;"), "{output}");
}

#[test]
fn union_members_attributes_and_trailing_punctuation() {
    let output = stable(
        "#[first]# #[second]# union<T,E> Result[Success<T>,Failed<E>,]; unit P{a:i32;b:i32;}; func main(){f(1,2,);var p=One{1,2,};}",
    );
    assert!(
        output.starts_with(
            "#[first]#\n#[second]#\nunion<T, E> Result[\n    Success<T>,\n    Failed<E>,\n];"
        ),
        "{output}"
    );
    assert!(output.contains("f(1, 2,);"), "{output}");
    assert!(output.contains("One {1, 2,};"), "{output}");
}

#[test]
fn comments_and_nested_generic_closers_stay_stable() {
    let nested = stable("func main(){add(One{1, // nested\n2});}");
    assert!(nested.contains("add(One {\n"), "{nested}");
    stable(
        "func f(first:i32,// param\nsecond:i32){var x:Box<Box<ReallyLongTypeName> > =g();var p=One{1, // element\n2};}",
    );
    stable("func main(){var a=[[aaaaaaaaaa,bbbbbbbbbb,cccccccccc],[1,2]];}");
    stable("func main(){for i in [0,3]{if(i<1){}else if(i<2){}else{}}}");
}
