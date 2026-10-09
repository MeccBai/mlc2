use super::*;

#[test]
fn unions_construct_match_and_specialize() {
    for source in [
        "union Number[i32,f64]; func main()->i32{var x=Number{1};match(y=@mut x){i32=>{$y=2;},f64=>{}}return 0;}",
        "unit Success<T>{pub value:T;};unit Failed<E>{pub error:E;};union<T,E> Result[Success<T>,Failed<E>];func main(){var x=Result<i32,i32>{Success<i32>{1}};match(y=@x){Success<i32> =>{var n=y->value;},Failed<i32> =>{var n=y->error;}}}",
        "union Value[i32,f64];pub Value::func inspect(self)->i32{match(y=@self){i32=>{return $y;},f64=>{return 0;}}}func main()->i32{var x=Value{1};return x.inspect();}",
    ] {
        valid(source);
    }
}

#[test]
fn union_errors_submit_without_panics() {
    for source in [
        "union V[i32,i32];func main(){}",
        "union<T,E> V[T,E];func main(){var x=V<i32,i32>{1};}",
        "union V[i32,f64];func main(){var x=V{true};}",
        "union V[i32,f64];func main(){var x=V{};}",
        "union V[i32,f64];func main(){var x=V{1};match(y=x){i32=>{},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(y=@x){i32=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(y=@x){i32=>{},i32=>{},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(y=@x){i32=>{},bool=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(y=@x){i32=>{$y=2;},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(y=@mut x){i32=>{x=V{2};},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};match(x){_=>{}}}",
        "union V[i32,f64];func main(){match(y=@V{1}){i32=>{},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};var r=@x;match(y=@r){i32=>{},f64=>{}}}",
        "union V[i32,f64];func main(){var x=V{1};var r=@mut x;match(y=@mut x){i32=>{$r=V{2};},f64=>{}}}",
    ] {
        assert!(build(source).config.is_poisoned(), "{source}");
    }
}
