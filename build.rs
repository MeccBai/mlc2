fn main() {
    let llvm = r"F:\Develop\scoop\apps\llvm-dev\current";

    println!("cargo:rustc-link-search=native={llvm}\\lib");
    println!("cargo:rustc-link-lib=dylib=LLVM-C");
} 