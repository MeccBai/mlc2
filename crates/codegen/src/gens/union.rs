use super::{
    IrGenerator,
    instruction::{Instruction, IrValue, LlvmType, TypedValue},
};
use crate::ast::{symbols::PackageSymbolTable, types::UnitType};

impl IrGenerator {
    pub(super) fn variant_deconstruct(
        unit: &UnitType,
        next: &mut usize,
        code: &mut Vec<Instruction>,
        package: &PackageSymbolTable,
    ) {
        let tag_slot = *next;
        let tag = tag_slot + 1;
        let payload = tag + 1;
        *next += 3;
        for (target, field) in [(tag_slot, 0), (payload, 1)] {
            code.push(Instruction::Gep {
                target,
                ty: Self::unit_lowering(unit, package),
                pointer: IrValue::Reg(0),
                indices: vec![
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(0),
                    },
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(field),
                    },
                ],
                inbounds: false,
            });
        }
        code.push(Instruction::Load {
            target: tag,
            ty: LlvmType::Int(32),
            pointer: IrValue::Reg(tag_slot),
            align: None,
        });
        let body = unit
            .deconstruct
            .as_ref()
            .expect("generated union destructor");
        code.push(Instruction::Switch {
            value: TypedValue {
                ty: LlvmType::Int(32),
                value: IrValue::Reg(tag),
            },
            default: "destroy.end".into(),
            cases: body
                .members
                .iter()
                .map(|member| {
                    (
                        IrValue::Integer(member.index as i128),
                        format!("destroy.{}", member.index),
                    )
                })
                .collect(),
        });
        for member in &body.members {
            code.push(Instruction::BasicBlock {
                label: format!("destroy.{}", member.index),
            });
            Self::append_drop(member.ty, payload, next, code, package);
            code.push(Instruction::Branch {
                label: "destroy.end".into(),
            });
        }
        code.push(Instruction::BasicBlock {
            label: "destroy.end".into(),
        });
    }
}
