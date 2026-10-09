use super::BackendError;
use llvm_sys::target::*;
use std::sync::Mutex;

/// Initialize only the requested architecture, once per process and family.
pub(super) fn initialize(triplet: &str) -> Result<(), BackendError> {
    let family = match triplet.split('-').next() {
        Some("x86_64" | "i386" | "i486" | "i586" | "i686") => "x86",
        Some("aarch64" | "arm64") => "aarch64",
        _ => {
            return Err(BackendError::Target(format!(
                "unsupported triplet: {triplet}"
            )));
        }
    };
    static INITIALIZED: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut initialized = INITIALIZED
        .lock()
        .map_err(|_| BackendError::Target("target initialization lock poisoned".into()))?;
    if !initialized.contains(&family) {
        unsafe {
            if family == "x86" {
                LLVMInitializeX86TargetInfo();
                LLVMInitializeX86Target();
                LLVMInitializeX86TargetMC();
                LLVMInitializeX86AsmPrinter();
                LLVMInitializeX86AsmParser();
            } else {
                LLVMInitializeAArch64TargetInfo();
                LLVMInitializeAArch64Target();
                LLVMInitializeAArch64TargetMC();
                LLVMInitializeAArch64AsmPrinter();
                LLVMInitializeAArch64AsmParser();
            }
        }
        initialized.push(family);
    }
    Ok(())
}
