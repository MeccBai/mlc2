use llvm_sys::{
    core::*,
    ir_reader::LLVMParseIRInContext2,
    prelude::*,
    target::*,
    target_machine::*,
};

use std::{
    ffi::{CStr, CString},
    ptr,
};

pub fn emit_obj(ir: &str, output: &str) -> Result<(), String> {
    unsafe {
        
        LLVM_InitializeAllTargetInfos();
        LLVM_InitializeAllTargets();
        LLVM_InitializeAllTargetMCs();
        LLVM_InitializeAllAsmPrinters();


        let context = LLVMContextCreate();

        let buffer_name = CString::new("mlc").unwrap();

        let buffer = LLVMCreateMemoryBufferWithMemoryRangeCopy(
            ir.as_ptr() as *const _,
            ir.len(),
            buffer_name.as_ptr(),
        );

        let mut module: LLVMModuleRef = ptr::null_mut();
        let mut error = ptr::null_mut();

        let failed = LLVMParseIRInContext2(
            context,
            buffer,
            &mut module,
            &mut error,
        );

        LLVMDisposeMemoryBuffer(buffer);

        if failed != 0 {
            let message = if error.is_null() {
                "unknown LLVM IR parse error".to_string()
            } else {
                let msg = CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned();

                LLVMDisposeMessage(error);
                msg
            };

            LLVMContextDispose(context);

            return Err(message);
        }


        let triple = LLVMGetDefaultTargetTriple();

        LLVMSetTarget(module, triple);

        let mut target = ptr::null_mut();
        let mut target_error = ptr::null_mut();

        if LLVMGetTargetFromTriple(
            triple,
            &mut target,
            &mut target_error,
        ) != 0
        {
            let msg = CStr::from_ptr(target_error)
                .to_string_lossy()
                .into_owned();

            LLVMDisposeMessage(target_error);
            LLVMDisposeMessage(triple);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);

            return Err(msg);
        }


        let cpu = CString::new("generic").unwrap();
        let features = CString::new("").unwrap();

        let machine = LLVMCreateTargetMachine(
            target,
            triple,
            cpu.as_ptr(),
            features.as_ptr(),
            LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
            LLVMRelocMode::LLVMRelocDefault,
            LLVMCodeModel::LLVMCodeModelDefault,
        );

        if machine.is_null() {
            LLVMDisposeMessage(triple);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);

            return Err("failed to create LLVM TargetMachine".into());
        }

        let data_layout = LLVMCreateTargetDataLayout(machine);
        LLVMSetModuleDataLayout(module, data_layout);
        LLVMDisposeTargetData(data_layout);

        let output = CString::new(output).unwrap();
        let mut emit_error = ptr::null_mut();

        let failed = LLVMTargetMachineEmitToFile(
            machine,
            module,
            output.as_ptr() as *mut _,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut emit_error,
        );

        if failed != 0 {
            let msg = CStr::from_ptr(emit_error)
                .to_string_lossy()
                .into_owned();

            LLVMDisposeMessage(emit_error);
            LLVMDisposeTargetMachine(machine);
            LLVMDisposeMessage(triple);
            LLVMDisposeModule(module);
            LLVMContextDispose(context);

            return Err(msg);
        }

        LLVMDisposeTargetMachine(machine);
        LLVMDisposeMessage(triple);
        LLVMDisposeModule(module);
        LLVMContextDispose(context);

        Ok(())
    }
}