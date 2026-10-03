use super::BackendError;
use crate::manifest::X86_64_PC_WINDOWS_MSVC;
use llvm_sys::target::*;
use std::sync::Mutex;

/// Initialize only the currently supported target family, once per process.
pub(super) fn initialize(triplet: &str) -> Result<(), BackendError> {
    if triplet != X86_64_PC_WINDOWS_MSVC {
        return Err(BackendError::Target(format!(
            "unsupported triplet: {triplet}"
        )));
    }
    static INITIALIZED: Mutex<bool> = Mutex::new(false);
    let mut initialized = INITIALIZED
        .lock()
        .map_err(|_| BackendError::Target("target initialization lock poisoned".into()))?;
    if !*initialized {
        unsafe {
            LLVMInitializeX86TargetInfo();
            LLVMInitializeX86Target();
            LLVMInitializeX86TargetMC();
            LLVMInitializeX86AsmPrinter();
            LLVMInitializeX86AsmParser();
        }
        *initialized = true;
    }
    Ok(())
}
