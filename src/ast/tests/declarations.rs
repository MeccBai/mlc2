use super::*;

valid_case!(empty_module, "");
valid_case!(
    comments_and_whitespace,
    "// heading\n /* comment */ func main() {} // end\n"
);
valid_case!(
    variable_qualifiers_and_explicit_types,
    "func main() { var a:i32 = 1; val b:i32 = a; const c:i32 = 3; var flag:bool = true; var real:f64 = 1.5; }"
);
valid_case!(
    global_variables_visible_in_functions,
    "global var counter:i32 = 0; global const limit:i32 = 10; func main() { var copy = counter; var maximum = limit; }"
);
valid_case!(
    forward_function_call,
    "func main() { var value = later(1); } func later(value:i32) -> i32 { return value; }"
);
valid_case!(
    declaration_only_and_variadic_symbols,
    "func external(...) -> i32; func mixed(value:i32,...) -> i32; func main() { var result = external(1,2); }"
);
valid_case!(
    export_and_attributes,
    "#[c_abi]# export func entry(value:i32) -> i32 { return value; } export unit Data { pub value:i32; }; export enum State { Ready, Busy, };"
);
valid_case!(
    unit_members_and_typed_initializer,
    "unit Point { pub x:i32; y:i32; }; func main() { var point = Point{1,2}; var x = point.x; }"
);
valid_case!(
    untyped_initializer_with_declared_unit,
    "unit Point { x:i32; y:i32; }; func main() { var point:Point = {1,2}; }"
);
valid_case!(
    enum_declaration_without_payload,
    "enum State { Ready, Busy, }; func main() {}"
);
valid_case!(
    receiver_free_interface_declaration,
    "unit Point {}; Point::func create() -> i32 { return 1; } func main() {}"
);
valid_case!(
    interfaces_have_independent_generic_contexts,
    "unit Point {}; Point::func<T> choose(value:$T) -> $T { return value; } func<T> choose(value:T) -> T { return value; } func main() { var p = choose<i32>(1); }"
);

syntax_case!(
    variable_initialization_is_mandatory,
    "func main() { var a:i32; }"
);
syntax_case!(
    val_initialization_is_mandatory,
    "func main() { val a:i32; }"
);
syntax_case!(no_parameter_after_variadic, "func wrong(..., value:i32);");
syntax_case!(no_duplicate_variadic_markers, "func wrong(...,...);");
syntax_case!(parameter_requires_postfix_type, "func wrong(i32 value);");
syntax_case!(no_exported_global_variable, "export global var a:i32 = 1;");
syntax_case!(no_api_global_variable, "api global const a:i32 = 1;");

semantic_case!(unknown_member_type, "unit Point { x:Missing; };");
semantic_case!(unknown_parameter_type, "func wrong(value:Missing) {}");
semantic_case!(unknown_return_type, "func wrong() -> Missing {}");
semantic_case!(unknown_interface_owner, "Missing::func wrong() {}");
semantic_case!(duplicate_function_parameter, "func wrong(a:i32,a:i32) {}");
syntax_case!(public_free_function_is_illegal, "pub func wrong() {}");
syntax_case!(api_unit_is_illegal, "api unit Wrong {};");
#[test]
fn attribute_groups_preserve_names_and_no_longer_use_array_brackets() {
    let ast = valid("#[c_abi, demo,]# #[extra]# export func main() {}");
    let index = ast.symbols.functions.get_by_name(&"main".into()).unwrap();
    assert_eq!(
        ast.symbols.functions.get(index).attributes,
        ["c_abi", "demo", "extra"]
    );
    invalid_syntax("[[c_abi]] func main() {}");
    invalid_syntax("#[c_abi] func main() {}");
}
