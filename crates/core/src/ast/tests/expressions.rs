use super::*;
use crate::ast::{
    expression::{Expression, InitialList},
    statement::Statement,
    types::ValueType,
};

valid_case!(
    arithmetic_and_parentheses,
    "func main() { var a = 1 + 2 * (3 - 4); var b = -a; var c = a / 2 % 3; }"
);
valid_case!(
    bitwise_and_shift_operators,
    "func main() { var a = (1 << 2) | (4 & 3) ^ 5; var b = ~a; var c = b >> 1; }"
);
valid_case!(
    comparison_and_logical_operators,
    "func main() { var a = (1 < 2) && (3 >= 2); var b = !a || false; var c = 1 != 2; }"
);
valid_case!(
    array_and_string_initialization,
    "func main() { var a = [1,2,3]; var first = a[0]; var text = \"hello\"; var ch = text[1]; }"
);
valid_case!(
    multidimensional_array_access,
    "func main() { var grid = [[1,2],[3,4]]; var value = grid[0][1]; }"
);
valid_case!(
    reference_and_explicit_mutable_reference,
    "func main() { var a = 1; var p:$i32 = @a; var q:$mut i32 = @mut a; var b = $p; var c = $q; }"
);
valid_case!(
    reference_to_immutable_value,
    "func main() { val a = 1; var p:$i32 = @a; var value = $p; }"
);
valid_case!(
    nested_reference,
    "func main() { var a = 1; var p:$i32 = @a; var q:$$i32 = @p; var value = $$q; }"
);
valid_case!(
    direct_and_indirect_member_access,
    "unit Point { pub x:i32; }; func main() { var p = Point{1}; var r = @p; var direct = p.x; var indirect = r->x; }"
);
valid_case!(
    plain_function_and_nested_calls,
    "func identity(x:i32) -> i32 { return x; } func main() { var a = identity(identity(1)); }"
);
valid_case!(
    pipeline_free_function,
    "func identity(x:i32) -> i32 { return x; } func main() { var value = 1 |> identity |> identity; }"
);
valid_case!(
    pipeline_argument_group,
    "func add(a:i32,b:i32) -> i32 { return a + b; } func main() { var value = {1,2} |> add; }"
);

semantic_case!(unknown_variable, "func main() { var a = missing; }");
semantic_case!(unknown_function, "func main() { missing(); }");
semantic_case!(
    unknown_member,
    "unit Point { x:i32; }; func main() { var p = Point{1}; var x = p.missing; }"
);
semantic_case!(
    indirect_access_requires_reference,
    "unit Point { x:i32; }; func main() { var p = Point{1}; var x = p->x; }"
);
semantic_case!(
    index_requires_array,
    "func main() { var a = 1; var x = a[0]; }"
);
semantic_case!(
    index_requires_integer,
    "func main() { var a = [1,2]; var x = a[true]; }"
);
semantic_case!(
    dereference_requires_reference,
    "func main() { var a = 1; var b = $a; }"
);
semantic_case!(
    mutable_reference_rejects_val,
    "func main() { val a = 1; var p = @mut a; }"
);
semantic_case!(
    mutable_reference_rejects_literal,
    "func main() { var p = @mut 1; }"
);
semantic_case!(
    const_rejects_function_call,
    "func value() -> i32 { return 1; } func main() { const a = value(); }"
);
semantic_case!(
    explicit_type_rejects_bool_integer_mismatch,
    "func main() { var a:i32 = true; }"
);

syntax_case!(
    member_pipeline_is_forbidden,
    "func main() { var a = 1 |> object.method(); }"
);
syntax_case!(
    empty_index_is_forbidden,
    "func main() { var a = [1]; var value = a[]; }"
);

#[test]
fn variable_value_qualifiers_survive_source_to_ast() {
    let ast = valid("func main() { var a = 1; val b = 2; const c = 3; }");
    for (statement, expected) in
        main_body(&ast)
            .iter()
            .zip([ValueType::Flex, ValueType::Final, ValueType::Constant])
    {
        let Statement::VariableDecl(variable) = statement else {
            panic!("variable expected");
        };
        assert_eq!(variable.var_type.value_type(&ast.symbols.types), expected);
    }
    assert_eq!(main_body(&ast).len(), 3);
}

#[test]
fn typed_initializer_preserves_resolved_target() {
    let ast = valid("unit Point { x:i32; }; func main() { var point = Point{1}; }");
    let Statement::VariableDecl(variable) = &main_body(&ast)[0] else {
        panic!("variable expected");
    };
    let Expression::InitListE(InitialList::List {
        onwer: Some(owner),
        values,
    }) = &*variable.init_val
    else {
        panic!("typed initializer expected");
    };
    assert_eq!(*owner, variable.var_type);
    assert_eq!(values.len(), 1);
}
