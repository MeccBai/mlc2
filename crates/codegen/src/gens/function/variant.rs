use super::*;
use crate::ast::statement::VariantMatch;

impl FunctionGenerator<'_> {
    pub(super) fn variant_match(&mut self, statement: &VariantMatch) {
        let value = self.expression(&statement.value);
        let pointer = self.load(&value);
        let llvm_type = IrGenerator::type_lowering(statement.owner, self.package);
        let tag_slot = self.allocate();
        self.code.push(Instruction::Gep {
            target: tag_slot,
            ty: llvm_type.clone(),
            pointer: pointer.clone(),
            indices: vec![
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(0),
                },
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(0),
                },
            ],
            inbounds: false,
        });
        let tag = self.allocate();
        self.code.push(Instruction::Load {
            target: tag,
            ty: LlvmType::Int(32),
            pointer: IrValue::Reg(tag_slot),
            align: None,
        });
        let payload = self.allocate();
        self.code.push(Instruction::Gep {
            target: payload,
            ty: llvm_type,
            pointer,
            indices: vec![
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(0),
                },
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(1),
                },
            ],
            inbounds: false,
        });
        let end = self.label("variant.end");
        let invalid = self.label("variant.invalid");
        let labels = statement
            .branches
            .iter()
            .map(|_| self.label("variant.arm"))
            .collect::<Vec<_>>();
        self.code.push(Instruction::Switch {
            value: TypedValue {
                ty: LlvmType::Int(32),
                value: IrValue::Reg(tag),
            },
            default: invalid.clone(),
            cases: statement
                .branches
                .iter()
                .zip(&labels)
                .map(|((tag, _, _), label)| (IrValue::Integer(*tag as i128), label.clone()))
                .collect(),
        });
        self.terminated = true;
        let mut falls = false;
        for ((_, variable, body), label) in statement.branches.iter().zip(labels) {
            self.block(label);
            let mark = self.variables.position();
            let storage = self.storage(LlvmType::Ptr);
            self.code.push(Instruction::Store {
                pointer: IrValue::Reg(storage.reg.unwrap()),
                value: TypedValue {
                    ty: LlvmType::Ptr,
                    value: IrValue::Reg(payload),
                },
                align: None,
            });
            self.bind(variable.name.clone(), variable.var_type, storage);
            falls |= self.scope(body, end.clone());
            self.bindings.remove(&variable.name);
            self.variables.restore(mark);
        }
        self.block(invalid);
        self.code.push(Instruction::Unreachable);
        self.terminated = true;
        if falls {
            self.block(end);
        }
    }
}
