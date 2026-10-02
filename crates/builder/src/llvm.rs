mod config;
mod emit;
mod error;
mod link;
mod target;

use crate::manifest::{EXE_SUFFIX, LLD_COFF, LLD_ELF};
pub use config::BackendConfig;
pub use error::{BackendError, DiagnosticLine, DiagnosticSeverity, LinkDiagnostic};
use llvm_sys::target_machine::*;
use std::{
    ffi::{CString, OsString},
    path::Path,
};

/// Owns one target machine; modules and contexts are isolated per emit operation.
pub struct IrCompiler {
    config: BackendConfig,
    triplet: CString,
    machine: LLVMTargetMachineRef,
}

impl IrCompiler {
    pub fn init(triplet: &str) -> Result<Self, BackendError> {
        Self::with_config(triplet, BackendConfig::current()?)
    }

    pub fn with_config(triplet: &str, config: BackendConfig) -> Result<Self, BackendError> {
        target::initialize(triplet)?;
        let triplet = CString::new(triplet).map_err(|e| BackendError::Config(e.to_string()))?;
        let mut target = std::ptr::null_mut();
        let mut message = std::ptr::null_mut();
        unsafe {
            if LLVMGetTargetFromTriple(triplet.as_ptr(), &mut target, &mut message) != 0 {
                return Err(BackendError::Target(emit::message(message)));
            }
            let machine = LLVMCreateTargetMachine(
                target,
                triplet.as_ptr(),
                c"generic".as_ptr(),
                c"".as_ptr(),
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault,
                LLVMRelocMode::LLVMRelocDefault,
                LLVMCodeModel::LLVMCodeModelDefault,
            );
            if machine.is_null() {
                return Err(BackendError::Target(
                    "could not create target machine".into(),
                ));
            }
            Ok(Self {
                config,
                triplet,
                machine,
            })
        }
    }

    pub fn config(&self) -> &BackendConfig {
        &self.config
    }

    pub fn emit(&self, ir: String, output: &Path) -> Result<(), BackendError> {
        emit::object(self.machine, &self.triplet, &ir, output)
    }

    /// Arguments include objects, output path, SDK libraries and any linker script.
    pub fn link(&self, arguments: &[OsString]) -> Result<LinkDiagnostic, BackendError> {
        let windows = self.triplet.to_bytes().windows(7).any(|s| s == b"windows");
        let program = self.config.tools.join(format!(
            "{}{}",
            if windows { LLD_COFF } else { LLD_ELF },
            EXE_SUFFIX
        ));
        let mut args = vec![if windows {
            OsString::from(format!("/libpath:{}", self.config.lib.display()))
        } else {
            OsString::from("-L")
        }];
        if !windows {
            args.push(self.config.lib.as_os_str().to_owned());
        }
        args.extend_from_slice(arguments);
        link::link(&program, &args)
    }
}

impl Drop for IrCompiler {
    fn drop(&mut self) {
        unsafe {
            LLVMDisposeTargetMachine(self.machine);
        }
    }
}

#[cfg(test)]
mod tests;
