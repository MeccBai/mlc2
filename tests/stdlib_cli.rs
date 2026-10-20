#![cfg(windows)]
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
static COMPILER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn compile(source: &str, run: bool) -> (tempfile::TempDir, Output) {
    // Each invocation generates helpers in the same standard-library cache.
    let _guard = COMPILER_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.m2"), source).unwrap();
    fs::write(root.path().join("config.toml"), "").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args([
            if run { "run" } else { "ll" },
            "main.m2",
            "--config",
            "config.toml",
            "--lib-dir",
        ])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("lib"))
        .output()
        .unwrap();
    (root, result)
}
fn success(result: &Output) {
    assert!(
        result.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn standard_library_hides_writer_and_validates_macros() {
    for (source, message) in [
        (
            "import std;func main(){std::print!(\"{} {}\",1);}",
            "placeholders",
        ),
        (
            "import std;func main(){std::println!(\"{}\");}",
            "placeholders",
        ),
        (
            "import std::fmt;func main(){var x:std::fmt::FormatWriter=0;}",
            "Unknown type",
        ),
        (
            "import std::fmt;func main(){std::fmt::signed(1);}",
            "Unknown function",
        ),
        (
            "import std;func main(){var x=std::format!(\"{} {}\",1);}",
            "placeholders",
        ),
        (
            "import std;func main(){var x=std::format!(\"{\",1);}",
            "format!",
        ),
        (
            "import std;func main(){var x=std::format!(\"}\");}",
            "format!",
        ),
        (
            "import std;func main(){var pattern:std::ConstStringView=\"{}\";var x=std::format!(pattern,1);}",
            "compile-time string",
        ),
        (
            "import std;unit Missing{};func main(){var x=std::format!(\"{}\",Missing{});}",
            "does not satisfy",
        ),
        (
            "import std::fmt;using hidden=std::fmt::FormatWriter;func main(){var x:hidden=0;}",
            "Unknown type",
        ),
    ] {
        let (_, result) = compile(source, false);
        assert!(!result.status.success(), "{source}");
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains(message), "{source}\n{stderr}");
    }
}
#[test]
fn standard_aliases_views_and_custom_macros_generate_valid_ir() {
    let (root, result) = compile(
        r#"
import std;
import std::base;
import std::fmt;
using format=std::format;
unit Point{pub x:i32;};
pub Point::func format(self)->std::String{return format!("Point({})",self->x);}
func main()->i32{
    var view:std::base::ConstStringView="中\0文";
    var s:std::String=std::String::from("hello");
    s.append(" world");
    var result:std::String=format!("{} {} {} {}",Point{42},cast<f32>(1.5),cast<u64>(18446744073709551615),false);
    return 0;
}"#,
        false,
    );
    success(&result);
    let ir = fs::read_to_string(root.path().join("build/ll/std/fmt.ll")).unwrap();
    assert!(ir.contains("declare i32 @\"snprintf\""), "{ir}");
    assert!(!ir.contains("define i32 @\"snprintf\""));
}
#[test]
#[ignore = "Requires bundled ld.lld and runtime beside mlc"]
fn strings_and_format_macros_run_with_expected_values() {
    let (_, result) = compile(
        r#"
import std;
import std::base;
import std::fmt;
import c_std::io;
using format=std::format;
global var calls:i32=0;
func next()->i32{calls=calls+1;return calls;}
unit Point{pub x:i32;};
pub Point::func format(self)->std::String{return format!("Point({})",self->x);}
func main()->i32{
    var empty=std::String::new();
    if(!empty.is_empty()){return 1;}
    var view:std::base::ConstStringView="中\0文";
    if(view.length!=cast<u64>(7)){return 2;}
    var text:std::String=std::String::from("hello");
    text.append(" world");
    for i in [0,100]{text.append("!");}
    if(text.len()!=cast<u64>(111)){return 3;}
    var copy=text.clone();
    if(copy.len()!=text.len()){return 4;}
    const pattern="{}|{}|{}|{}|{}|{}";
    var result:std::String=format!(pattern,Point{42},cast<f32>(1.5),cast<u64>(18446744073709551615),false,"中",next());
    if(calls!=1){return 5;}
    c_std::io::printf(const_c_str("%s\n"),result.as_ptr());
    var escaped=std::format!("{{ok}} and no arguments");
    c_std::io::printf(const_c_str("%s\n"),escaped.as_ptr());
    return 0;
}"#,
        true,
    );
    success(&result);
    let output = String::from_utf8(result.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        output.contains("Point(42)|1.5|18446744073709551615|false|中|1\n"),
        "{output}"
    );
    assert!(output.contains("{ok} and no arguments\n"), "{output}");
}

#[test]
#[ignore = "Requires bundled ld.lld and runtime beside mlc"]
fn print_macros_write_formatted_bytes_and_newlines() {
    let (_, result) = compile(
        r#"
import std;
using print=std::print;
using println=std::println;
global var calls:i32=0;
func next()->i32{calls=calls+1;return calls;}
func main()->i32 {
    print!("prefix:");
    println!("{} {} {} 100%",next(),"中文",true);
    println!();
    std::print!("{{{}}}",42);
    std::println!("\0end");
    if(calls!=1){return 1;}
    return 0;
}
"#,
        true,
    );
    success(&result);
    let output = String::from_utf8(result.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        output.contains("prefix:1 中文 true 100%\n\n{42}\0end\n"),
        "{output:?}"
    );
}

#[test]
fn native_str_rejects_runtime_construction_and_implicit_pointers() {
    for source in [
        r#"func main(){var text:str=42;}"#,
        r#"func main(){var invalid="x"+"y";}"#,
        r#"func change(text:str){text="changed";}func main(){}"#,
        r#"func main(){var text:str="a";text="b";}"#,
        r#"func main(){var text:str="a";var writable=@mut text;}"#,
        r#"func main(){var bytes:[i8:2]=[65,0];var text:str=bytes;}"#,
        r#"func main(){var pointer:const $i8="hello";}"#,
        r#"func take(value:const $i8){}func main(){take("hello");}"#,
        r#"func dynamic()->str{return "hello";}func main(){var text:str=dynamic();}"#,
        r#"import std;func main(){var dynamic=std::String::from("hello");var text:str=dynamic.view();}"#,
        r#"import std;macro func custom!(pattern:str,args:std::macro::Args<std::fmt::Display>){std::println!("ok");}func dynamic()->str{return "x";}func main(){custom!(dynamic());}"#,
    ] {
        let (_, result) = compile(source, false);
        assert!(!result.status.success(), "unexpectedly accepted: {source}");
        let errors = String::from_utf8_lossy(&result.stderr);
        assert!(!errors.is_empty(), "{source}: missing diagnostic");
        assert!(!errors.contains("panicked"), "{source}: {errors}");
    }
}
#[test]
#[ignore = "Requires bundled ld.lld and runtime beside mlc"]
fn native_str_literals_parameters_lengths_and_explicit_c_pointers_run() {
    let (_, result) = compile(
        r#"
import std;
import c_std::io;
using Text=str;
global val global_pattern:str="{}:{}";
macro func line!(pattern:str,args:std::macro::Args<std::fmt::Display>) {
    std::println!(pattern);
}
func length(text:str)->u64{return str::len(text);}
func borrowed_length(text:$str)->u64{return str::len($text);}
func pointer(text:str)->const $i8{return const_c_str(text);}
func emit(text:str){c_std::io::printf(const_c_str("%s"),pointer(text));}
func main()->i32 {
    var text:str="中\0文";
    val alias:Text=text;
    const size:u64=str::len("中\0文");
    if(size!=cast<u64>(7)){return 1;}
    if(length(alias)!=size){return 2;}
    val borrowed=@text;
    if(borrowed_length(borrowed)!=size){return 6;}
    val c_text=const_c_str(alias);
    if(c_text[3]!=cast<i8>(0)){return 3;}
    if(c_text[7]!=cast<i8>(0)){return 4;}
    if(str::len("")!=cast<u64>(0)){return 5;}
    emit("C: ");
    var pattern:Text="{}:{}";
    std::println!(pattern,"native",42);
    std::println!(global_pattern,"global",7);
    line!("nested");
    std::println!("{}",text);
    return 0;
}
"#,
        true,
    );
    success(&result);
    let output = String::from_utf8(result.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(
        output.contains("C: native:42\nglobal:7\nnested\n中\0文\n"),
        "{output:?}"
    );
}

#[test]
#[ignore = "Requires bundled ld.lld and runtime beside mlc"]
fn composed_numeric_and_display_constraints_run() {
    let (_, result) = compile(
        r#"
import std;
import std::fmt;
generic PrintableInteger = std::fmt::Display && (Wide && Integer);
generic Integer {std::generic::is_integer;}
generic Wide {std::generic::min_bits<32>;}
func<T:PrintableInteger> twice(value:T)->T{return value+value;}
func main()->i32 {
    std::println!("answer={}",twice(21));
    return 0;
}
"#,
        true,
    );
    success(&result);
    let output = String::from_utf8(result.stdout)
        .unwrap()
        .replace("\r\n", "\n");
    assert!(output.contains("answer=42\n"), "{output}");
}
