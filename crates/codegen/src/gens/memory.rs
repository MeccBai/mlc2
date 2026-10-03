//! Shared memory intrinsics and immutable literal resources.
use super::{
    IrGenerator,
    instruction::{Instruction, IrValue, LlvmType, TypedValue},
};
use std::fmt::Write;

pub(super) const MEMSET: &str = "llvm.memset.p0.i64";
pub(super) const MEMCPY: &str = "llvm.memcpy.p0.p0.i64";

pub(super) fn string_name(bytes: &[u8]) -> String {
    // Collision-free content key, deduplicating literals across functions/files.
    let mut name = String::from(".mlc.str.");
    for byte in bytes {
        write!(name, "{byte:02X}").expect("String write");
    }
    name
}
fn typed(ty: LlvmType, value: IrValue) -> TypedValue {
    TypedValue { ty, value }
}
pub(super) fn zero(destination: IrValue, size: usize) -> Instruction {
    intrinsic(
        MEMSET,
        vec![
            typed(LlvmType::Ptr, destination),
            typed(LlvmType::Int(8), IrValue::Integer(0)),
            typed(LlvmType::Int(64), IrValue::Integer(size as i128)),
            typed(LlvmType::Int(1), IrValue::Bool(false)),
        ],
    )
}
pub(super) fn copy(destination: IrValue, source: IrValue, size: usize) -> Instruction {
    intrinsic(
        MEMCPY,
        vec![
            typed(LlvmType::Ptr, destination),
            typed(LlvmType::Ptr, source),
            typed(LlvmType::Int(64), IrValue::Integer(size as i128)),
            typed(LlvmType::Int(1), IrValue::Bool(false)),
        ],
    )
}
fn intrinsic(name: &str, args: Vec<TypedValue>) -> Instruction {
    Instruction::Call {
        target: None,
        func: IrValue::Global(name.into()),
        return_type: LlvmType::Void,
        args,
        variadic_params: None,
    }
}
impl IrGenerator {
    pub(super) fn add_resources(&mut self, resources: &[Instruction], intrinsics: &[String]) {
        for resource in resources {
            let Instruction::StringConstant { name, .. } = resource else {
                unreachable!()
            };
            if self.remember_symbol(format!("literal::{name}")) {
                writeln!(self.header, "{resource}").expect("String write");
            }
        }
        for name in intrinsics {
            if name == "malloc" || name == "free" {
                if self.remember_symbol(name.clone()) {
                    let declaration = if name == "malloc" {
                        "declare ptr @malloc(i64)"
                    } else {
                        "declare void @free(ptr)"
                    };
                    let start = self.defines.len();
                    writeln!(self.defines, "{declaration}").expect("String write");
                    self.declarations
                        .push((name.clone(), start..self.defines.len()));
                }
                continue;
            }
            if self.remember_symbol(format!("intrinsic::{name}")) {
                let signature = match name.as_str() {
                    MEMSET => "ptr, i8, i64, i1 immarg",
                    MEMCPY => "ptr, ptr, i64, i1 immarg",
                    _ => super::error::fail("Unknown memory intrinsic"),
                };
                writeln!(
                    self.header,
                    "declare void {}({signature})",
                    IrValue::Global(name.clone())
                )
                .expect("String write");
            }
        }
    }
}
