//! Per-function storage, control-flow state and future cleanup snapshots.
use super::{
    IrGenerator, LlvmValue,
    error::{fail, unsupported},
    expr::ExpressionBindings,
    func::{LlvmFunc, ParameterMode},
    instruction::{Instruction, InstructionBuffer, IrValue, LlvmType, TypedValue},
    variable_stack::{ExitKind, ScopeExit, StackPosition, VariableStack},
};
use crate::ast::{
    Function, arena::TypeIndex, expression::Expression, statement::Statement,
    symbols::PackageSymbolTable,
};
use std::fmt::Write;

mod control;
mod statements;
#[cfg(test)]
mod tests;

pub struct GeneratedFunction {
    pub name: String,
    pub ir: String,
    pub variables: VariableStack,
    pub exits: Vec<ScopeExit>,
    pub calls: Vec<LlvmFunc>,
}

struct LoopTarget {
    break_label: String,
    continue_label: String,
    break_to: StackPosition,
    continue_to: StackPosition,
}

pub struct FunctionGenerator<'a> {
    package: &'a PackageSymbolTable,
    abi: LlvmFunc,
    next_reg: usize,
    next_label: usize,
    entry: Vec<Instruction>,
    code: Vec<Instruction>,
    bindings: ExpressionBindings,
    variables: VariableStack,
    exits: Vec<ScopeExit>,
    loops: Vec<LoopTarget>,
    terminated: bool,
    return_slot: Option<LlvmValue>,
}

impl<'a> FunctionGenerator<'a> {
    pub fn generate(function: &Function, package: &'a PackageSymbolTable) -> GeneratedFunction {
        let (abi, params, body) = match function {
            Function::Func(function) => {
                let symbol = package.get_function(function.symbol, false);
                (symbol.llvm_func(package), &symbol.params, &function.body)
            }
            Function::Interface(function) => {
                let symbol = package.get_interface(function.symbol, false);
                (symbol.llvm_func(package), &symbol.params, &function.body)
            }
        };
        let abi = abi.unwrap_or_else(|error| error.abort_generation());
        let mut generator = Self {
            next_reg: abi.parameters().count(),
            abi,
            package,
            next_label: 0,
            entry: vec![],
            code: vec![],
            bindings: ExpressionBindings::new(),
            variables: VariableStack::default(),
            exits: vec![],
            loops: vec![],
            terminated: false,
            return_slot: None,
        };
        generator.parameters(params);
        generator.statements(body);
        if !generator.terminated {
            if generator.return_slot.is_some() {
                // No definite-return check exists yet: never return uninitialized storage.
                generator.code.push(Instruction::Unreachable);
                generator.terminated = true;
            } else {
                generator.exit(
                    ExitKind::Return,
                    StackPosition::default(),
                    "function.return".into(),
                );
            }
        }
        generator.finish()
    }

    fn allocate(&mut self) -> usize {
        let reg = self.next_reg;
        self.next_reg = reg
            .checked_add(1)
            .unwrap_or_else(|| fail("Function register space exhausted"));
        reg
    }

    fn label(&mut self, prefix: &str) -> String {
        let id = self.next_label;
        self.next_label = id
            .checked_add(1)
            .unwrap_or_else(|| fail("Function label space exhausted"));
        format!("function.{prefix}.{id}")
    }

    fn block(&mut self, label: String) {
        if !self.terminated && !self.code.is_empty() {
            fail("Basic block was not terminated before opening another block");
        }
        self.code.push(Instruction::BasicBlock { label });
        self.terminated = false;
    }

    fn branch(&mut self, label: String) {
        if self.terminated {
            fail("Attempted to append a second block terminator");
        }
        self.code.push(Instruction::Branch { label });
        self.terminated = true;
    }

    fn exit(&mut self, kind: ExitKind, to: StackPosition, target: String) {
        let from = self.variables.position();
        self.variables.between(from, to);
        self.exits.push(ScopeExit {
            kind,
            from,
            to,
            target: target.clone(),
        });
        // Future RAII redirects this branch into a cleanup chain keyed by this snapshot.
        self.branch(target);
    }

    fn scope(&mut self, statements: &[Statement], target: String) -> bool {
        let mark = self.variables.position();
        self.statements(statements);
        let falls_through = !self.terminated;
        if falls_through {
            self.exit(ExitKind::Scope, mark, target);
        }
        let names: Vec<_> = self
            .variables
            .between(self.variables.position(), mark)
            .iter()
            .map(|v| v.name.clone())
            .collect();
        for name in names {
            self.bindings.remove(&name);
        }
        self.variables.restore(mark);
        falls_through
    }

    fn expression(&mut self, expr: &Expression) -> LlvmValue {
        let (next, mut value) =
            IrGenerator::expression_expand(self.next_reg, expr, self.package, &self.bindings);
        self.next_reg = next;
        // Fixed storage belongs to entry, not a loop body: otherwise each
        // iteration allocates another slot until the function returns.
        for instruction in value.code.drain(..) {
            if matches!(instruction, Instruction::Alloca { count: None, .. }) {
                self.entry.push(instruction);
            } else {
                self.code.push(instruction);
            }
        }
        value
    }

    fn load(&mut self, value: &LlvmValue) -> IrValue {
        let reg = value
            .reg
            .unwrap_or_else(|| fail("Void value used by a statement"));
        if value.in_reg {
            return IrValue::Reg(reg);
        }
        let target = self.allocate();
        self.code.push(Instruction::Load {
            target,
            ty: value.ty.clone(),
            pointer: IrValue::Reg(reg),
            align: None,
        });
        IrValue::Reg(target)
    }

    fn storage(&mut self, ty: LlvmType) -> LlvmValue {
        let target = self.allocate();
        self.entry.push(Instruction::Alloca {
            target,
            ty: ty.clone(),
            count: None,
            align: None,
        });
        LlvmValue {
            code: vec![],
            ty,
            reg: Some(target),
            in_reg: false,
        }
    }

    fn bind(&mut self, name: String, ty: TypeIndex, value: LlvmValue) {
        if self.bindings.contains_key(&name) {
            fail("Duplicate active variable binding during generation");
        }
        self.variables.push(name.clone(), ty, value.clone());
        self.bindings.insert(name, value);
    }

    fn parameters(&mut self, params: &[(TypeIndex, String)]) {
        let mut parameter = 0;
        if let Some(hidden) = self.abi.result().hidden.first() {
            self.return_slot = Some(LlvmValue {
                code: vec![],
                ty: hidden.ty.clone(),
                reg: Some(parameter),
                in_reg: false,
            });
            parameter += 1;
        } else if self.abi.result().ty != LlvmType::Void {
            self.return_slot = Some(self.storage(self.abi.result().ty.clone()));
        }
        if let Some(receiver) = self.abi.receiver() {
            let ty = receiver.source;
            let value = LlvmValue {
                code: vec![],
                ty: receiver.lowered[0].ty.clone(),
                reg: Some(parameter),
                in_reg: false,
            };
            self.bind("self".into(), ty, value);
            parameter += 1;
        }
        let mappings = self.abi.params().to_vec();
        for ((ty, name), mapping) in params
            .iter()
            .filter(|(_, name)| name != "...")
            .zip(mappings)
        {
            if mapping.lowered.len() != 1 {
                unsupported("split parameter definition");
            }
            let lowered = &mapping.lowered[0];
            let value = if lowered.mode == ParameterMode::Direct {
                let storage = self.storage(lowered.ty.clone());
                self.entry.push(Instruction::Store {
                    pointer: IrValue::Reg(storage.reg.unwrap()),
                    value: TypedValue {
                        ty: lowered.ty.clone(),
                        value: IrValue::Reg(parameter),
                    },
                    align: None,
                });
                storage
            } else {
                LlvmValue {
                    code: vec![],
                    ty: lowered.ty.clone(),
                    reg: Some(parameter),
                    in_reg: false,
                }
            };
            self.bind(name.clone(), *ty, value);
            parameter += 1;
        }
    }

    fn finish(mut self) -> GeneratedFunction {
        self.block("function.return".into());
        let result = if self.abi.result().ty == LlvmType::Void {
            None
        } else {
            let slot = self
                .return_slot
                .clone()
                .unwrap_or_else(|| fail("Missing return storage"));
            Some(TypedValue {
                ty: slot.ty.clone(),
                value: self.load(&slot),
            })
        };
        self.code.push(Instruction::Return { value: result });
        let mut ir = format!(
            "define {} {}(",
            self.abi.result().ty,
            IrValue::Global(self.abi.name().into())
        );
        for (index, parameter) in self.abi.parameters().enumerate() {
            if index != 0 {
                ir.push_str(", ");
            }
            write!(ir, "{parameter} %r{index}").expect("String write");
        }
        if self.abi.is_variadic() {
            if self.abi.parameters().next().is_some() {
                ir.push_str(", ");
            }
            ir.push_str("...");
        }
        ir.push_str(") {\nentry:\n");
        let calls = self
            .code
            .iter()
            .filter_map(|instruction| match instruction {
                Instruction::MappedCall { func, .. } => Some((**func).clone()),
                _ => None,
            })
            .collect();
        InstructionBuffer {
            instructions: self.entry,
        }
        .write_to(&mut ir)
        .expect("String write");
        InstructionBuffer {
            instructions: self.code,
        }
        .write_to(&mut ir)
        .expect("String write");
        ir.push_str("}\n");
        GeneratedFunction {
            name: self.abi.name().into(),
            ir,
            variables: self.variables,
            exits: self.exits,
            calls,
        }
    }
}
