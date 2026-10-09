use super::*;

#[test]
fn explicit_function_addresses_and_indirect_calls_are_checked() {
    for source in [
        "unit<F> Box{pub f:F;}func id(x:i32)->i32{return x;}func<T> wrap(f:func(T)->T)->Box<func(T)->T>{return Box<func(T)->T>{f};}func main()->i32{var x=wrap<i32>(std::function(id));return x.f(42);}",
        "func max(a:i32,b:i32)->i32{return a;}func apply(f:func(i32,i32)->i32)->i32{return f(1,2);}func main()->i32{var p:func(i32,i32)->i32=std::function(max);return apply(p);}",
        "func noop(){}func main(){var p:func()=std::function(noop);p();}",
        "func id(x:i32)->i32{return x;}func make()->func(i32)->i32{return std::function(id);}func main()->i32{var p=make();return p(42);}",
        "unit<T> Holder{pub f:func(T)->T;}func id(x:i32)->i32{return x;}func<T> apply(f:func(T)->T,x:T)->T{return f(x);}func main()->i32{var h=Holder<i32>{std::function(id)};return apply(h.f,42);}",
        "func id(x:i32)->i32{return x;}union F[func(i32)->i32,i32];func main()->i32{var x=F{std::function(id)};match(y=@x){func(i32)->i32=>{return ($y)(42);},i32=>{return $y;}}}",
        "func max(a:i32,b:i32)->i32 { return a; } func main()->i32 { var p = std::function(max); return p(1,2); }",
        "func<T> max(a:T,b:T)->T { return a; } func main()->u32 { var p = std::function<u32>(max); return p(cast<u32>(1),cast<u32>(2)); }",
        "func a(x:i32)->i32{return x;} func b(x:i32)->i32{return x;} func main(){var p=std::function(a);p=std::function(b);p(1);}",
        "func a(x:i32)->i32{return x;} func main()->i32 {var p=[std::function(a)];return p[0](4);}",
        "func a(x:i32)->i32{return x;} func<F> invoke(f:F)->i32{return f(3);} func main()->i32 {return invoke(std::function(a));}",
    ] {
        valid(source);
    }
}

#[test]
fn function_pointer_errors_are_semantic_not_panics() {
    for source in [
        "func f(x:i32)->i32{return x;}func main(){var p:func(bool)->i32=std::function(f);}",
        "func f(x:i32)->i32{return x;}func main(){var p:func(i32)=std::function(f);}",
        "#[c_abi]# func f(x:i32)->i32{return x;}func main(){var p:func(i32)->i32=std::function(f);}",
        "func<T> f(x:T)->T{return x;} func main(){var p=std::function(f);}",
        "func f(x:i32){} func main(){var p=std::function<i32>(f);}",
        "func f(x:i32){} func main(){var p=std::function(f);p();}",
        "func f(x:i32){} func main(){var p=std::function(f);p(true);}",
        "func f(x:i32){} func main(){var p=std::function(f);p<i32>(1);}",
        "func f(x:i32){} func main(){var p=std::function(1);}",
        "func f(x:i32){} func main(){var p=std::function(f,f);}",
        "func main(){var x=1;var p=std::function(x);}",
        "func f(){} func main(){var p=std::function(f);var x=p+p;}",
        "func f(){} func main(){var p=std::function(f);var x=-p;}",
        "func f(){} func main(){var p=std::function(f);var x=$p;}",
        "func f(){} func main(){var p=std::function(f);match(p){_=>{}}}",
        "func f(x:i32){} func g(x:bool){} func main(){var p=std::function(f);p=std::function(g);}",
        "func f()->i32{return 1;} func g()->bool{return true;} func main(){var p=std::function(f);p=std::function(g);}",
        "func f(x:i32){} func g(x:i32,...){} func main(){var p=std::function(f);p=std::function(g);}",
        "func f(p:$i32){} func g(p:$mut i32){} func main(){var p=std::function(f);p=std::function(g);}",
        "func main(){var p=std::function(cast);}",
        "func a(){} func<F> f(p:F){var array:[F:2]=[p];} func main(){f(std::function(a));}",
        "unit Holder<F>{callback:F;}; func a(){} func<F> f(p:F){var array:[Holder<F>:2]=[Holder<F>{p}];} func main(){f(std::function(a));}",
        "func f(x:i32){} #[c_abi]# func g(x:i32){} func main(){var p=std::function(f);p=std::function(g);}",
        "unit U{}; pub U::func f(self){} func main(){var p=std::function(U::f);}",
    ] {
        assert!(build(source).config.is_poisoned(), "{source}");
    }
}
