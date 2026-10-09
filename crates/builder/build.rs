#[allow(dead_code)]
#[path = "src/manifest.rs"]
mod manifest;

fn main() {
    println!(
        "cargo:rustc-env=MLC_HOST_TRIPLET={}",
        if std::env::var("HOST").unwrap() == manifest::X86_64_PC_WINDOWS_MSVC {
            manifest::X86_64_PC_WINDOWS_GNU.to_owned()
        } else {
            std::env::var("HOST").unwrap()
        }
    );
    let lib = std::path::Path::new(manifest::LLVM_INSTALL_DIR).join(manifest::LIB_DIR);

    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=dylib={}", manifest::LLVM_C_LIBRARY);
}
