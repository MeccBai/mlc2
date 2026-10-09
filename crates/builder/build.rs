#[allow(dead_code)]
#[path = "src/manifest.rs"]
mod manifest;

fn main() {
    println!(
        "cargo:rustc-env=MLC_HOST_TRIPLET={}",
        manifest::system_triplet(&std::env::var("TARGET").expect("Cargo provides TARGET"))
    );
    let lib = std::path::Path::new(manifest::LLVM_INSTALL_DIR).join(manifest::LIB_DIR);

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib={}", manifest::LLVM_C_LIBRARY);
}
