use super::*;

#[test]
fn exported_function_pointer_signatures_resolve_across_files() {
    let mut package = PackageSymbolTable::new();
    let mut owner = ast(
        1,
        "owner",
        "export func apply(f:func(i32)->i32,x:i32)->i32{return f(x);} export func<T> generic_apply(f:func(T)->T,x:T)->T{return f(x);}",
    );
    publish(&mut package, &mut owner);
    analyze(&mut package, &mut owner);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut caller = ast(
        2,
        "caller",
        "func id(x:i32)->i32{return x;}func main()->i32{var p:func(i32)->i32=std::function(id);var a=owner::apply(p,1);return owner::generic_apply(p,a);}",
    );
    analyze(&mut package, &mut caller);
}
