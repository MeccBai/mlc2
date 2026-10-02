use super::*;
use crate::ast::{expression::FuncCall, symbols::EnumBool};
use crate::gens::func::SymbolIr;

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
        let values: Vec<_> = call
            .args
            .iter()
            .enumerate()
            .map(|(index, expression)| {
                self.expression_expected(expression, mappings.get(index).map(|m| m.source))
                    .value
            })
            .collect();
        let (next, value) = func.call_values(self.next, &values, self.symbols);
        self.next = next;
        Lowered {
            value,
            signed: returns.is_some_and(|ty| self.signed(ty)),
        }
    }
}
