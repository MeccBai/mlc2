mod codeview;
mod config;
mod emit;
mod error;
mod link;
mod target;

pub use config::BackendConfig;
pub use error::{BackendError, DiagnosticLine, DiagnosticSeverity, LinkDiagnostic};
use llvm_sys::target_machine::*;
use mlc_manifest as manifest;
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
        Self::with_level(triplet, config, LLVMCodeGenOptLevel::LLVMCodeGenLevelNone)
    }

    pub fn with_optimization(triplet: &str, optimize: bool) -> Result<Self, BackendError> {
        Self::with_level(
            triplet,
            BackendConfig::current()?,
            if optimize {
                LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault
            } else {
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone
            },
        )
    }

    fn with_level(
        triplet: &str,
        config: BackendConfig,
        level: LLVMCodeGenOptLevel,
    ) -> Result<Self, BackendError> {
        Self::with_relocation(triplet, config, level, LLVMRelocMode::LLVMRelocDefault)
    }
    fn with_relocation(
        triplet: &str,
        config: BackendConfig,
        level: LLVMCodeGenOptLevel,
        relocation: LLVMRelocMode,
    ) -> Result<Self, BackendError> {
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
                level,
                relocation,
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

    pub fn target_layout(&self) -> mlc_core::ast::config::TargetLayout {
        unsafe {
            let data = llvm_sys::target_machine::LLVMCreateTargetDataLayout(self.machine);
            let context = llvm_sys::core::LLVMContextCreate();
            let pointer = llvm_sys::core::LLVMPointerTypeInContext(context, 0);
            let layout = mlc_core::ast::config::TargetLayout {
                ptr_size: llvm_sys::target::LLVMPointerSize(data) as usize,
                ptr_align: llvm_sys::target::LLVMABIAlignmentOfType(data, pointer) as usize,
            };
            llvm_sys::core::LLVMContextDispose(context);
            llvm_sys::target::LLVMDisposeTargetData(data);
            layout
        }
    }

    pub fn with_o3(triplet: &str) -> Result<Self, BackendError> {
        Self::with_relocation(
            triplet,
            BackendConfig::current()?,
            LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
            LLVMRelocMode::LLVMRelocPIC,
        )
    }
    pub fn emit_o3(&self, ir: String, output: &Path) -> Result<(), BackendError> {
        emit::object(self.machine, &self.triplet, &ir, output, true)
    }

    pub fn config(&self) -> &BackendConfig {
        &self.config
    }

    pub fn emit(&self, ir: String, output: &Path) -> Result<(), BackendError> {
        emit::object(self.machine, &self.triplet, &ir, output, false)?;
        if ir.contains("!llvm.dbg.cu")
            && self
                .triplet
                .to_bytes()
                .windows(7)
                .any(|bytes| bytes == b"windows")
        {
            codeview::preserve_step_over(output).map_err(BackendError::Codegen)?;
        }
        Ok(())
    }

    /// Arguments include objects, output path, SDK libraries and any linker script.
    pub fn link(&self, arguments: &[OsString]) -> Result<LinkDiagnostic, BackendError> {
        // Explicit LLVM triples stay at the backend boundary; manifest only maps enums forward.
        let triple = self.triplet.to_bytes();
        let abi = if triple.ends_with(b"-windows-msvc") {
            manifest::Abi::Msvc
        } else if triple.ends_with(b"-darwin") {
            manifest::Abi::Darwin
        } else {
            manifest::Abi::Gnu
        };
        let windows = abi == manifest::Abi::Msvc;
        let program = self
            .config
            .executable
            .parent()
            .ok_or_else(|| BackendError::Config("executable has no parent directory".into()))?
            .join(manifest::TOOLS_DIR)
            .join(format!("{}{}", abi.linker(), manifest::EXE_SUFFIX));
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
