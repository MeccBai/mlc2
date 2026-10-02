use super::*;

semantic_case!(
    c_unit_cannot_have_generics,
    "#[c_abi]# unit C<T> { value:T; }; func main() {}"
);
semantic_case!(
    c_function_cannot_have_generics,
    "#[c_abi]# func<T> call(value:T); func main() {}"
);
semantic_case!(
    c_interface_cannot_have_generics,
    "unit Owner {}; #[c_abi]# Owner::func<T> call(value:T); func main() {}"
);
semantic_case!(
    c_unit_cannot_have_interface,
    "#[c_abi]# unit C {}; C::func method() {} func main() {}"
);
semantic_case!(
    c_unit_cannot_have_generic_interface,
    "#[c_abi]# unit C {}; C::func<T> method(value:T) {} func main() {}"
);
semantic_case!(
    c_function_rejects_language_unit,
    "unit Native {}; #[c_abi]# func call(value:Native); func main() {}"
);
semantic_case!(
    c_function_rejects_language_unit_reference,
    "unit Native {}; #[c_abi]# func call(value:$Native); func main() {}"
);
semantic_case!(
    c_interface_rejects_language_unit_parameter,
    "unit Owner {}; unit Native {}; #[c_abi]# Owner::func call(value:Native); func main() {}"
);
valid_case!(
    c_function_accepts_c_unit,
    "#[c_abi]# unit C { pub value:i32; }; #[c_abi]# func call(value:C); func main() {}"
);
valid_case!(
    c_function_accepts_c_unit_reference,
    "#[c_abi]# unit C {}; #[c_abi]# func call(value:$C); func main() {}"
);
valid_case!(
    c_function_accepts_primitives_and_variadic_marker,
    "#[c_abi]# func call(value:i32,...); func main() {}"
);
valid_case!(
    language_function_can_receive_c_unit,
    "#[c_abi]# unit C {}; func call(value:C) {} func main() {}"
);
valid_case!(
    language_generics_remain_allowed,
    "unit Native<T> { value:T; }; func<T> call(value:T) {} func main() {}"
);
