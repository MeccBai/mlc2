#[allow(dead_code)]
#[path = "src/manifest.rs"]
mod manifest;

fn main() {
    println!("cargo:rerun-if-env-changed=SCOOP");
    println!(
        "cargo:rustc-env=MLC_HOST_TRIPLET={}",
        manifest::system_triplet(&std::env::var("TARGET").expect("Cargo provides TARGET"))
    );
    let scoop = std::env::var_os("SCOOP").expect("SCOOP environment variable is not set");
    let lib = std::path::Path::new(&scoop)
        .join("apps")
        .join("llvm-dev")
        .join("current")
        .join(manifest::LIB_DIR);
    if !lib.join("LLVM-C.lib").is_file() {
        panic!("LLVM-C.lib was not found under {}", lib.display());
    }

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib={}", manifest::LLVM_C_LIBRARY);
}
