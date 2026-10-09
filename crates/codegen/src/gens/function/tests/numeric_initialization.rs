use super::*;

#[test]
fn scalar_constants_are_lowered_in_destination_types_without_default_i32_truncation() {
    let ir = emit(
        "func wide() -> u64 { return 4294967296; } func main() { var x:u64 = 4294967296; var y:i8 = -128; var f:f32 = 1.5; var n:u32 = 42.0; var a:[u64:2] = [4294967296,1]; }",
        "numeric-constants",
    );
    assert!(ir.contains("store i64 4294967296"));
    assert!(!ir.contains("store i32 4294967296"));
    assert!(ir.contains("store i8 -128"));
    assert!(ir.contains("store i32 42"));
}

#[test]
fn runtime_numeric_initializers_emit_lossless_conversions() {
    let ir = emit(
        "func widen(a:i8,b:u8,c:i32,d:f32) { var x:i64 = a; var y:i32 = b; var f:f64 = c; var real:f64 = d; var array:[f64:1] = [c]; }",
        "numeric-widening",
    );
    assert!(ir.contains("sext i8"));
    assert!(ir.contains("zext i8"));
    assert!(ir.contains("sitofp i32"));
    assert!(ir.contains("fpext float"));
}
