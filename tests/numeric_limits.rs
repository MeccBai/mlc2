use std::{fs, process::Command};
fn ll(source: &str, expected: bool) -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), source).unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["ll", "main.m2", "--default", "--config", "config.toml"])
        .output()
        .unwrap();
    assert_eq!(
        result.status.success(),
        expected,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = if expected {
        fs::read_to_string(
            root.path()
                .join("build/ll/default")
                .join(mlc_manifest::Target::default().triplet())
                .join("main.ll"),
        )
        .unwrap()
    } else {
        String::from_utf8(result.stderr).unwrap()
    };
    (root, text)
}
#[test]
fn numeric_limits_are_typed_compile_time_literals_without_optimization() {
    let mut source = String::new();
    for ty in [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ] {
        for operation in ["max", "min"] {
            source.push_str(&format!(
                "export func {ty}_{operation}()->{ty}{{return std::{ty}::{operation}();}}\n"
            ));
        }
    }
    for ty in ["f32", "f64"] {
        for operation in ["min_positive", "epsilon"] {
            source.push_str(&format!(
                "export func {ty}_{operation}()->{ty}{{return std::{ty}::{operation}();}}\n"
            ));
        }
    }
    source.push_str("func main()->i32{const limit=std::u32::max();var n:u32=limit;return 0;}");
    let (root, ir) = ll(&source, true);
    for (ty, max, min) in [
        ("i8", "127", "-128"),
        ("i16", "32767", "-32768"),
        ("i32", "2147483647", "-2147483648"),
        ("i64", "9223372036854775807", "-9223372036854775808"),
        ("i8", "255", "0"),
        ("i16", "65535", "0"),
        ("i32", "4294967295", "0"),
        ("i64", "18446744073709551615", "0"),
    ] {
        assert!(ir.contains(&format!("store {ty} {max}")), "{ir}");
        assert!(ir.contains(&format!("store {ty} {min}")), "{ir}");
    }
    assert!(!ir.lines().any(|line| line.contains("call ")
        && (line.contains("::max") || line.contains("::min") || line.contains("::epsilon"))));
    mlc_builder::llvm::IrCompiler::init(mlc_manifest::Target::default().triplet())
        .unwrap()
        .emit(ir, &root.path().join("limits.obj"))
        .unwrap();
}
#[test]
fn numeric_limits_reject_arguments_type_arguments_and_implicit_narrowing() {
    for source in [
        "func main(){var n=std::u32::max(1);}",
        "func main(){var n=std::i32::max<i32>();}",
        "func main(){var n:i32=std::u32::max();}",
        "func main(){var n=std::i128::max();}",
        "func main(){var n=std::u32::epsilon();}",
    ] {
        ll(source, false);
    }
}

#[test]
fn bit_builtins_emit_scalar_operations_and_float_bitcasts_without_calls() {
    let (root, ir) = ll(
        "export func mask(a:f32,b:i32)->u32{return std::bit::and(a,b);}export func toggle(a:f64,b:u64)->u64{return std::bit::xor(a,b);}export func invert(a:f32)->u32{return std::bit::not(a);}export func combine(a:i32,b:u32)->u32{return std::bit::or(a,b);}export func left(a:i32,b:i32)->u32{return std::bit::shl(a,b);}export func right(a:i32,b:i32)->u32{return std::bit::shr(a,b);}func main()->i32{return 0;}",
        true,
    );
    for operation in [
        "bitcast float",
        "bitcast double",
        "and i32",
        "xor i64",
        "xor i32",
        "or i32",
        "shl i32",
        "lshr i32",
    ] {
        assert!(ir.contains(operation), "missing {operation}: {ir}");
    }
    assert!(
        !ir.lines()
            .any(|line| line.contains("call ") && line.contains("std::bit")),
        "{ir}"
    );
    mlc_builder::llvm::IrCompiler::init(mlc_manifest::Target::default().triplet())
        .unwrap()
        .emit(ir, &root.path().join("bits.obj"))
        .unwrap();
}
