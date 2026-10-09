use super::*;
use crate::ast::{expression::FuncCall, symbols::EnumBool};
use crate::gens::func::SymbolIr;

impl Expander<'_> {
    pub(super) fn call(&mut self, call: &FuncCall) -> Lowered {
        if let EnumBool::False(index) = call.func
            && call.callee.is_none()
        {
            let symbol = self.symbols.get_function(index, false);
            if let Some(kind) = symbol.builtin() {
                return self.builtin(kind, symbol, &call.args[0]);
            }
        }
        let (func, returns) = match call.func {
            EnumBool::True(index) => {
                let symbol = self.symbols.get_interface(index, false);
                (symbol.llvm_func(self.symbols), symbol.ret_type)
            }
            EnumBool::False(index) => {
                let symbol = self.symbols.get_function(index, false);
                (symbol.llvm_func(self.symbols), symbol.ret_type)
            }
        };
        let func = func.unwrap_or_else(|error| error.abort_generation());
        let mappings: Vec<_> = func.receiver().into_iter().chain(func.params()).collect();
        let mut pointer = call
            .callee
            .as_ref()
            .map(|callee| self.expression(callee).value);
        let target = pointer.as_mut().map(|pointer| self.load(pointer));
        let values: Vec<_> = call
            .args
            .iter()
            .enumerate()
            .map(|(index, expression)| {
                self.expression_expected(expression, mappings.get(index).map(|m| m.source))
                    .value
            })
            .collect();
        let (next, mut value) = func.call_values(self.next, &values, self.symbols);
        if let Some(target) = target {
            let index = value
                .code
                .iter()
                .rposition(|instruction| matches!(instruction, Instruction::MappedCall { .. }))
                .unwrap_or_else(|| fail("Missing mapped indirect call"));
            let Instruction::MappedCall {
                func,
                target: result,
                args,
            } = value.code.remove(index)
            else {
                unreachable!()
            };
            value.code.insert(
                index,
                Instruction::IndirectCall {
                    func,
                    callee: target,
                    target: result,
                    args,
                },
            );
            let mut code = pointer.unwrap().code;
            code.append(&mut value.code);
            value.code = code;
        }
        self.next = next;
        Lowered {
            value,
            signed: returns.is_some_and(|ty| self.signed(ty)),
        }
    }
}
