use super::BackendError;
use llvm_sys::target::*;
use std::sync::Mutex;

use crate::manifest::{
    AARCH64_NONE_ELF, RISCV32_NONE_ELF, THUMBV7EM_NONE_EABI, X86_64_PC_WINDOWS_MSVC,
};

/// Registration is global in LLVM. Serialize and initialize each selected family once.
pub(super) fn initialize(triplet: &str) -> Result<(), BackendError> {
    let family = match triplet {
        X86_64_PC_WINDOWS_MSVC => 0,
        THUMBV7EM_NONE_EABI => 1,
        AARCH64_NONE_ELF => 2,
        RISCV32_NONE_ELF => 3,
        _ => {
            return Err(BackendError::Target(format!(
                "unsupported triplet: {triplet}"
            )));
        }
    };
    static INITIALIZED: Mutex<[bool; 4]> = Mutex::new([false; 4]);
    let mut initialized = INITIALIZED
        .lock()
        .map_err(|_| BackendError::Target("target initialization lock poisoned".into()))?;
    if initialized[family] {
        return Ok(());
    }
    unsafe {
        let calls: [unsafe extern "C" fn(); 5] = match family {
            0 => [
                LLVMInitializeX86TargetInfo,
                LLVMInitializeX86Target,
                LLVMInitializeX86TargetMC,
                LLVMInitializeX86AsmPrinter,
                LLVMInitializeX86AsmParser,
            ],
            1 => [
                LLVMInitializeARMTargetInfo,
                LLVMInitializeARMTarget,
                LLVMInitializeARMTargetMC,
                LLVMInitializeARMAsmPrinter,
                LLVMInitializeARMAsmParser,
            ],
            2 => [
                LLVMInitializeAArch64TargetInfo,
                LLVMInitializeAArch64Target,
                LLVMInitializeAArch64TargetMC,
                LLVMInitializeAArch64AsmPrinter,
                LLVMInitializeAArch64AsmParser,
            ],
            _ => [
                LLVMInitializeRISCVTargetInfo,
                LLVMInitializeRISCVTarget,
                LLVMInitializeRISCVTargetMC,
                LLVMInitializeRISCVAsmPrinter,
                LLVMInitializeRISCVAsmParser,
            ],
        };
        for call in calls {
            call();
        }
    }
    initialized[family] = true;
    Ok(())
}
