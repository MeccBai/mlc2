use super::*;
use crate::ast::{
    statement::{Assignment, ReturnStatement},
    types::{CompileType, base_type::DataType},
};
use crate::gens::instruction::Cast;

impl FunctionGenerator<'_> {
    pub(super) fn statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            if self.terminated {
                break;
            }
            self.statement(statement);
        }
    }

    fn statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Poison => fail("Poison statement reached function generation"),
            Statement::VariableDecl(variable) => {
                let signed = self.source_signed(&variable.init_val);
                let value = self.expression(&variable.init_val);
                let storage =
                    self.storage(IrGenerator::type_lowering(variable.var_type, self.package));
                self.store(&storage, value, signed);
                // Initialization must complete before this variable enters the cleanup history.
                self.bind(variable.name.clone(), variable.var_type, storage);
            }
            Statement::Assignment(Assignment { variable, value }) => {
                let signed = self.source_signed(value);
                let target = self.expression(variable);
                if target.in_reg {
                    fail("Assignment target does not identify storage");
                }
                let value = self.expression(value);
                self.store(&target, value, signed);
            }
            Statement::Expression(expression) => {
                self.expression(expression);
            }
            Statement::FuncCall(call) => {
                self.expression(&Expression::FuncCallE(call.clone()));
            }
            Statement::ReturnBlock(ReturnStatement { value }) => {
                match (self.return_slot.clone(), value) {
                    (Some(storage), Some(expression)) => {
                        let signed = self.source_signed(expression);
                        let value = self.expression(expression);
                        self.store(&storage, value, signed);
                    }
                    (None, None) => {}
                    _ => fail("Return statement does not match its checked signature"),
                }
                self.exit(
                    ExitKind::Return,
                    StackPosition::default(),
                    "function.return".into(),
                );
            }
            Statement::Break | Statement::Continue => {
                let target = self
                    .loops
                    .last()
                    .unwrap_or_else(|| fail("Loop exit without a loop"));
                let (kind, to, label) = if matches!(statement, Statement::Break) {
                    (ExitKind::Break, target.break_to, target.break_label.clone())
                } else {
                    (
                        ExitKind::Continue,
                        target.continue_to,
                        target.continue_label.clone(),
                    )
                };
                self.exit(kind, to, label);
            }
            Statement::AnonymousBlock(block) => {
                let next = self.label("scope.end");
                if self.scope(&block.statements, next.clone()) {
                    self.block(next);
                }
            }
            Statement::IfBlock(statement) => self.if_statement(statement),
            Statement::WhileBlock(statement) => self.while_statement(statement),
            Statement::ForBlock(statement) => self.for_statement(statement),
            Statement::MatchBlock(statement) => self.match_statement(statement),
        }
    }

    fn source_signed(&self, expression: &Expression) -> bool {
        IrGenerator::expression_type(expression, self.package).is_some_and(|ty| {
            !ty.is_empty()
                && matches!(self.package.get_type(ty).unqualified(), CompileType::Base(base)
                if base.data_type() == DataType::Integer && base.signed())
        })
    }

    fn store(&mut self, storage: &LlvmValue, value: LlvmValue, signed: bool) {
        let operand = self.load(&value);
        let operand = if storage.ty != value.ty {
            let cast = match (&value.ty, &storage.ty) {
                (LlvmType::Int(from), LlvmType::Int(to)) if from > to => Cast::Trunc,
                (LlvmType::Int(_), LlvmType::Int(_)) if signed => Cast::SExt,
                (LlvmType::Int(_), LlvmType::Int(_)) => Cast::ZExt,
                (LlvmType::Float, LlvmType::Double) => Cast::FPExt,
                (LlvmType::Double, LlvmType::Float) => Cast::FPTrunc,
                _ => fail("Unsupported checked storage conversion"),
            };
            let target = self.allocate();
            self.code.push(Instruction::Cast {
                target,
                op: cast,
                value: TypedValue {
                    ty: value.ty,
                    value: operand,
                },
                to: storage.ty.clone(),
            });
            IrValue::Reg(target)
        } else {
            operand
        };
        self.code.push(Instruction::Store {
            pointer: IrValue::Reg(storage.reg.unwrap()),
            value: TypedValue {
                ty: storage.ty.clone(),
                value: operand,
            },
            align: None,
        });
    }

    pub(super) fn condition(
        &mut self,
        expression: &Expression,
        then_label: String,
        else_label: String,
    ) {
        let value = self.expression(expression);
        if value.ty != LlvmType::Int(1) {
            fail("Checked condition did not lower to i1");
        }
        let condition = self.load(&value);
        self.code.push(Instruction::ConditionalBranch {
            condition,
            then_label,
            else_label,
        });
        self.terminated = true;
    }
}
