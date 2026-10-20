use super::*;

#[test]
fn interface_visibility_keeps_public_and_export_independent() {
    for (visibility, public, exported) in [
        ("", false, false),
        ("pub", true, false),
        ("export", false, true),
        ("api", true, true),
    ] {
        for generic in [false, true] {
            let mut package = PackageSymbolTable::new();
            let signature = if generic {
                "<T> get(x:T) -> T { return x; }"
            } else {
                " get() -> i32 { return 1; }"
            };
            let mut owner = ast(
                1,
                "owner",
                &format!("export unit P {{}}; {visibility} P::func{signature}"),
            );
            let exports = owner.export(&mut package);
            assert!(
                !owner.config.is_poisoned(),
                "{:?}",
                owner.config.error_handle()
            );
            let name = "owner::P::get";
            let symbol = if exported {
                exports.searchable[name]
            } else {
                exports.inner[name]
            };
            assert_eq!(exports.searchable.contains_key(name), exported);
            assert_eq!(exports.inner.contains_key(name), !exported);
            let ExportSymbol::Interface {
                index,
                generic: actual_generic,
            } = symbol
            else {
                panic!("expected an interface")
            };
            assert_eq!(actual_generic, generic);
            let symbol = package.get_interface(index, generic);
            assert_eq!((symbol.public, symbol.exported), (public, exported));

            package.set_imports(FileId::new(2), [FileId::new(1)]);
            let call = if generic {
                "owner::P::get<i32>(1)"
            } else {
                "owner::P::get()"
            };
            let mut caller = ast(2, "caller", &format!("func main() {{ var x = {call}; }}"));
            caller.analysis(&mut package);
            assert_eq!(
                !caller.config.is_poisoned(),
                public && exported,
                "visibility={visibility}, generic={generic}: {:?}",
                caller.config.error_handle()
            );
        }
    }
}

#[test]
fn composed_constraints_keep_export_visibility_and_cross_file_signature_identity() {
    let mut package = PackageSymbolTable::new();
    let mut owner = ast(
        1,
        "owner",
        "generic Hidden{std::generic::is_integer;}export generic Integer=Hidden;export generic Reader{pub func read(self)->i32;}",
    );
    publish(&mut package, &mut owner);
    package.set_imports(FileId::new(2), [FileId::new(1)]);
    let mut caller = ast(
        2,
        "caller",
        "generic Same{pub func read(self)->i32;}generic Both=owner::Reader && Same;generic Number=owner::Integer;unit P{};pub P::func read(self)->i32{return 42;}func<T:Both> check(value:T){}func<T:Number> number(value:T){}func main(){check(P{});number(42);}",
    );
    publish(&mut package, &mut caller);
    analyze(&mut package, &mut caller);
    let file = package.file(FileId::new(2)).unwrap();
    let index = file.generics.requires.get_by_name(&"Both".into()).unwrap();
    assert_eq!(file.generics.requires.get(index).requires.len(), 1);
    package.set_imports(FileId::new(3), [FileId::new(1)]);
    let mut denied = ast(3, "denied", "generic Nope=owner::Hidden;");
    denied.export(&mut package);
    assert!(denied.config.is_poisoned());
}
