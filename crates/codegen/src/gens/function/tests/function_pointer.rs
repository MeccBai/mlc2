use super::*;

#[test]
fn declared_function_types_lower_without_phantom_definitions() {
    let ir = emit(
        "func max(a:i32,b:i32)->i32{return a;}func apply(f:func(i32,i32)->i32)->i32{return f(1,2);}func make()->func(i32,i32)->i32{return std::function(max);}func main()->i32{var p:func(i32,i32)->i32=make();return apply(p);}",
        "declared-function-pointer",
    );
    assert!(ir.contains("call i32 %r"));
    assert!(!ir.contains("@\".mlc.signature."));
}

#[test]
fn function_addresses_and_indirect_calls_produce_valid_llvm() {
    let ir = emit(
        "func max(a:i32,b:i32)->i32 { if(a>b){return a;}else{return b;} } func main()->i32 {var p=std::function(max);var list=[p];return list[0](2,3);}",
        "function-pointer",
    );
    assert!(ir.contains("bitcast ptr @"));
    assert!(ir.contains("call i32 %r"));
}

#[test]
fn indirect_calls_reuse_hidden_return_and_resource_parameter_abis() {
    let ir = emit(
        "unit U {a:i64;b:i64;}; func make(p:res $mut i32)->U{return U{1,2};} func main(p:res $mut i32){var f=std::function(make);var result=f(p);}",
        "function-pointer-abi",
    );
    assert!(ir.contains("call void %r"));
    assert!(ir.contains("store ptr zeroinitializer"));
}
