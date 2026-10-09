use super::*;

#[test]
fn union_payload_and_borrowed_match_lower_to_valid_llvm() {
    let ir = emit(
        "union Number[i32,f64];func f()->i32{var x=Number{1};match(y=@mut x){i32=>{$y=2;return $y;},f64=>{return 0;}}}",
        "union-match",
    );
    assert!(ir.contains("type { i32, [1 x i64] }"));
    assert!(ir.contains("switch i32"));
}

#[test]
fn generic_union_and_interface_lower_to_valid_llvm() {
    emit(
        "unit Success<T>{pub value:T;};unit Failed<E>{pub error:E;};union<T,E> Result[Success<T>,Failed<E>];func f()->i32{var x=Result<i32,i32>{Success<i32>{42}};match(y=@x){Success<i32> =>{return y->value;},Failed<i32> =>{return y->error;}}}",
        "generic-union",
    );
    emit(
        "union V[i32,f64];pub V::func read(self)->i32{match(y=@self){i32=>{return $y;},f64=>{return 0;}}}func f()->i32{var x=V{1};return x.read();}",
        "union-interface",
    );
}
