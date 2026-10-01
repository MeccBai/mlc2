use super::BackendError;
use llvm_sys::{
    analysis::*, core::*, ir_reader::LLVMParseIRInContext2, prelude::*, target::*,
    target_machine::*,
};
use std::{
    ffi::{CStr, CString},
    path::Path,
    ptr,
};

pub(super) unsafe fn message(raw: *mut std::ffi::c_char) -> String {
    if raw.is_null() {
        return "LLVM returned no diagnostic".into();
    }
    unsafe {
        let text = CStr::from_ptr(raw).to_string_lossy().into_owned();
        LLVMDisposeMessage(raw);
        text
    }
}

struct Module {
    context: LLVMContextRef,
    module: LLVMModuleRef,
}
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            if !self.module.is_null() {
                LLVMDisposeModule(self.module);
            }
            LLVMContextDispose(self.context);
        }
    }
}

pub(super) fn object(
    machine: LLVMTargetMachineRef,
    triplet: &CStr,
    ir: &str,
    output: &Path,
) -> Result<(), BackendError> {
    let path = output
        .to_str()
        .ok_or_else(|| BackendError::Config("LLVM output path must be UTF-8".into()))?;
    let mut path = CString::new(path)
        .map_err(|e| BackendError::Config(e.to_string()))?
        .into_bytes_with_nul();
    unsafe {
        let mut module = Module {
            context: LLVMContextCreate(),
            module: ptr::null_mut(),
        };
        let buffer = LLVMCreateMemoryBufferWithMemoryRangeCopy(
            ir.as_ptr().cast(),
            ir.len(),
            c"mlc".as_ptr(),
        );
        let mut error = ptr::null_mut();
        let failed = LLVMParseIRInContext2(module.context, buffer, &mut module.module, &mut error);
        // InContext2 borrows the buffer, unlike the deprecated consuming API.
        LLVMDisposeMemoryBuffer(buffer);
        if failed != 0 {
            return Err(BackendError::Codegen(message(error)));
        }
        LLVMSetTarget(module.module, triplet.as_ptr());
        let layout = LLVMCreateTargetDataLayout(machine);
        LLVMSetModuleDataLayout(module.module, layout);
        LLVMDisposeTargetData(layout);
        let mut error = ptr::null_mut();
        let invalid = LLVMVerifyModule(
            module.module,
            LLVMVerifierFailureAction::LLVMReturnStatusAction,
            &mut error,
        );
        let diagnostic = message(error);
        if invalid != 0 {
            return Err(BackendError::Codegen(diagnostic));
        }
        let mut error = ptr::null_mut();
        if LLVMTargetMachineEmitToFile(
            machine,
            module.module,
            path.as_mut_ptr().cast(),
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
        ) != 0
        {
            return Err(BackendError::Codegen(message(error)));
        }
        Ok(())
    }
}
