use super::*;


#[test]
fn exported_union_candidates_survive_cross_file_instantiation() {
    let mut package = PackageSymbolTable::new();
    let mut owner = ast(
        1,
        "owner",
        "export unit<T> Box{pub value:T;} export union<T> V[Box<T>,i32];export func<T> make(x:T)->V<T>{return V<T>{Box<T>{x}};}",
    );
    publish(&mut package, &mut owner);
    analyze(&mut package, &mut owner);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut caller = ast(
        2,
        "caller",
        "func main()->i32{var x=owner::make<i32>(42);match(y=@x){owner::Box<i32> =>{return y->value;},i32=>{return $y;}}}",
    );
    analyze(&mut package, &mut caller);
}
