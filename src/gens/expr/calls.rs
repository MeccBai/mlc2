use super::*;
use crate::ast::{expression::FuncCall, symbols::EnumBool};

impl Expander<'_> {
    pub(super) fn call(&mut self, call: &FuncCall) -> Lowered {
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
        if mappings.iter().any(|m| m.lowered.len() != 1) {
            unsupported("split ABI argument");
        }
        let mut code = vec![];
        let mut args = vec![];
        for (index, expression) in call.args.iter().enumerate() {
            let mut argument = self.expression(expression).value;
            let receiver_ref = mappings.get(index).is_some_and(|m| {
                m.lowered[0].mode == super::super::func::ParameterMode::Receiver
                    && argument.ty == LlvmType::Ptr
            });
            if let Some(mapping) = mappings.get(index) {
                if !receiver_ref && argument.ty != mapping.lowered[0].ty {
                    fail("Checked expression has inconsistent lowering types");
                }
            }
            let pointer = mappings.get(index).is_some_and(|m| {
                m.lowered[0].llvm_type() == LlvmType::Ptr
                    && m.lowered[0].mode != super::super::func::ParameterMode::Direct
            });
            let value = if receiver_ref {
                // A ref receiver already contains the owner's address, not the owner value.
                self.load(&mut argument)
            } else if pointer {
                self.address(&mut argument)
            } else {
                self.load(&mut argument)
            };
            code.extend(argument.code);
            args.push(TypedValue {
                ty: if pointer { LlvmType::Ptr } else { argument.ty },
                value,
            });
        }
        let (next, mut value) = func
            .call(self.next, &args)
            .unwrap_or_else(|error| fail(&format!("Expression lowering failed: {error}")));
        self.next = next;
        code.append(&mut value.code);
        value.code = code;
        Lowered {
            value,
            signed: returns.is_some_and(|ty| self.signed(ty)),
        }
    }
}
