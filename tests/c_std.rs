mod support;
use mlc_builder::artifacts;
use std::{collections::BTreeSet, fs};

fn declarations(target: &str) -> Vec<artifacts::Manifest> {
    let installation = support::library().parent().unwrap().to_owned();
    let mut result = Vec::new();
    for variant in ["universal", target] {
        for entry in fs::read_dir(installation.join("objects").join(variant).join("c_std")).unwrap()
        {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "sym") {
                result.push(artifacts::load(&path).unwrap().manifest);
            }
        }
    }
    result
}
#[test]
fn common_c_declarations_are_universal_and_windows_layouts_are_targeted() {
    let manifests = declarations("x86_64-pc-windows-msvc");
    let mut count = 0;
    for manifest in &manifests {
        assert_eq!(manifest.config.object, artifacts::ObjectMode::None);
        let module = &manifest.config.module_name;
        if [
            "c_std::runtime",
            "c_std::time",
            "c_std::lib_platform",
            "c_std::io_platform",
        ]
        .contains(&module.as_str())
        {
            assert_eq!(manifest.config.target, "x86_64-pc-windows-msvc");
        } else {
            assert!(manifest.config.target.is_empty(), "{module}");
        }
        count += manifest.symbols.functions.len();
    }
    assert!(count >= 190, "{count}");
    let function = |module: &str, name: &str| {
        manifests
            .iter()
            .find(|m| m.config.module_name == module)
            .unwrap()
            .symbols
            .functions
            .iter()
            .find(|f| f.name == name)
            .unwrap()
    };
    assert_eq!(function("c_std::string", "memcpy").params.len(), 3);
    assert_eq!(function("c_std::lib", "strtoll").params.len(), 3);
    for name in [
        "printf", "scanf", "puts", "getchar", "fread", "fwrite", "snprintf",
    ] {
        assert!(!function("c_std::io", name).name.is_empty());
    }
    assert_eq!(function("c_std::lib", "abs").params.len(), 1);
    assert_eq!(
        function("c_std::io", "fprintf").params.last().unwrap().name,
        "..."
    );
    assert_eq!(
        manifests
            .iter()
            .find(|m| m.config.module_name == "c_std::time")
            .unwrap()
            .symbols
            .units[0]
            .members
            .len(),
        9
    );
}

#[cfg(windows)]
#[test]
fn every_declared_function_links_to_ucrt() {
    use mlc_builder::llvm::IrCompiler;
    let symbols: BTreeSet<_> = declarations("x86_64-pc-windows-msvc")
        .iter()
        .flat_map(|m| m.symbols.functions.iter().map(|f| f.name.clone()))
        .collect();
    let mut ir = String::new();
    for name in &symbols {
        ir.push_str(&format!("declare void @\"{name}\"()\n"));
    }
    let names = symbols
        .iter()
        .map(|name| format!("ptr @\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    ir.push_str(&format!("@addresses = global [{} x ptr] [{names}]\ndefine i32 @main() {{\nentry:\n  %p = load volatile ptr, ptr @addresses\n  ret i32 0\n}}\n", symbols.len()));
    let root = tempfile::tempdir().unwrap();
    let object = root.path().join("all.obj");
    let exe = root.path().join("all.exe");
    IrCompiler::init("x86_64-pc-windows-msvc")
        .unwrap()
        .emit(ir, &object)
        .unwrap();
    let result = support::link(&[object], &exe);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(std::process::Command::new(exe).status().unwrap().success());
}

#[cfg(windows)]
#[test]
fn common_functions_import_compile_and_run() {
    run_common_functions("x86_64-pc-windows-msvc");
}

#[cfg(windows)]
#[test]
#[ignore = "Requires bundled GNU runtime and ld.lld beside the compiler"]
fn common_functions_run_with_gnu_runtime() {
    run_common_functions("x86_64-pc-windows-gnu");
}

#[cfg(windows)]
fn run_common_functions(target: &str) {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("main.m2"),
        r#"
import c_std::lib;
import c_std::io;
import c_std::string;
import c_std::ctype;
import c_std::math;
import c_std::runtime;
import c_std::time;
import c_std::lib_platform;
func main()->i32 {
    if(c_std::lib::atoi(const_c_str("42"))!=42){return 1;}
    var marker:i8=0;
    var end=@mut marker;
    if(c_std::lib::strtoll(const_c_str("123"),@mut end,10)!=cast<i64>(123)){return 2;}
    if(c_std::ctype::isdigit(53)==0){return 3;}
    if(c_std::ctype::toupper(97)!=65){return 4;}
    if(c_std::math::sqrt(81.0)!=9.0){return 5;}
    var p=c_std::lib::calloc(cast<u64>(16),cast<u64>(1));
    c_std::string::memcpy(p,const_c_str("hello"),cast<u64>(6));
    if(c_std::string::strlen(p)!=cast<u64>(5)){return 6;}
    if(c_std::string::strcmp(p,const_c_str("hello"))!=0){return 7;}
    var stream=c_std::io::tmpfile();
    if(c_std::io::fputs(p,stream)<0){return 9;}
    c_std::io::rewind(stream);
    c_std::string::memset(p,0,cast<u64>(16));
    if(c_std::io::fread(p,cast<u64>(1),cast<u64>(5),stream)!=cast<u64>(5)){return 10;}
    if(c_std::runtime::strcmp(p,const_c_str("hello"))!=0){return 11;}
    c_std::io::fclose(stream);
    c_std::lib::free(p);
    var now:i64=0;
    c_std::time::_time64(@mut now);
    if(now<=cast<i64>(0)){return 12;}
    var calendar=c_std::time::tm{0,0,0,0,0,0,0,0,0};
    if(c_std::time::_gmtime64_s(@mut calendar,@now)!=0){return 14;}
    if(calendar.tm_year<100){return 15;}
    if(c_std::lib_platform::strtol(const_c_str("99"),@mut end,10)!=99){return 13;}
    c_std::io::puts(const_c_str("c_std ok"));
    return 0;
}
"#,
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_mlc"))
        .current_dir(root.path())
        .args(["run", "main.m2", "--triplet", target, "--lib-dir"])
        .arg(support::library())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("c_std ok"));
}
