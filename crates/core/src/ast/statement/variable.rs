use std::{collections::HashMap, rc::Rc};

use crate::{
    ast::{
        arena::TypeIndex,
        config::Config,
        expression::Expression,
        statement::Variable,
        symbols::{EnumBool, SymbolTable},
        types::{ValueType, resolve_type},
    },
    diagnostic::error::IllegalUseError,
    parser::out::TempVar,
};

impl Variable {
    pub fn is_poisoned(&self) -> bool {
        self.init_val.is_poisoned()
    }

    pub fn poison() -> Rc<Self> {
        Rc::new(Self {
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::Poison),
        })
    }
    pub fn empty() -> Rc<Self> {
        Rc::new(Variable {
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::null()),
        })
    }

    pub fn new(
        config: &mut Config,
        temp_var: TempVar,
        symbols: &mut SymbolTable,
        context: Option<&mut crate::ast::symbols::StatementContext>,
    ) -> Rc<Self> {
        let var_span = temp_var.initializer.1;

        let ctxt = context.map(|ctxt| &*ctxt);
        let init_val = Expression::new(config, temp_var.initializer, symbols, ctxt);
        if init_val.is_poisoned() {
            return Self::poison();
        }
        let inferred = init_val.type_inference(config, symbols);

        let ty = match temp_var.ty {
            Some(ty) => match resolve_type(
                config,
                ty,
                symbols,
                ctxt.map(|context| context as &dyn crate::ast::types::TypeContext),
            ) {
                Some(ty) => {
                    if init_val.type_check(&ty, config, symbols) == false {
                        config.submit_error(
                            crate::diagnostic::error::CompileError::IllegalUse(
                                IllegalUseError::TypeMismatched {
                                    expected: symbols.types.get(ty).format(&symbols.types),
                                    found: symbols.types.get(inferred).format(&symbols.types),
                                },
                            ),
                            var_span,
                        );
                    }
                    ty
                }
                None => TypeIndex::empty(),
            },
            None => {
                if inferred.is_empty() {
                    config.submit_error(
                        crate::diagnostic::error::CompileError::Resolve(
                            crate::diagnostic::error::ResolveError::MissingType,
                        ),
                        var_span,
                    );
                }
                inferred
            }
        }
        .into_value_type(temp_var.value_type, &mut symbols.types);

        if temp_var.value_type == ValueType::Constant && !init_val.is_const(symbols) {
            config.submit_error(
                crate::diagnostic::error::CompileError::IllegalUse(
                    IllegalUseError::NonConstantInitializer,
                ),
                var_span,
            );
        }

        if config.is_poisoned() {
            return Self::poison();
        }
        let init_val = init_val.const_fold(config, symbols);
        Rc::new(Self {
            name: temp_var.name,
            var_type: ty,
            init_val: Box::new(init_val),
        })
    }

    /*
    let immut: Option<&HashMap<String, Rc<Variable>>> = context.map(|m| &*m);

                let init = match value {
                    Some(v) => Expression::new(config, v, symbols, immut),
                    None => Expression::null(),
                };

                let var_type = match ty {
                    Some(t) => match resolve_type(config, t, symbols) {
                        Some(ty) => ty,
                        None => TypeIndex::empty(),
                    },
                    None => init.type_inference(config, symbols),
                };

                if init.is_null() {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::VariableMustBeInitialized),
                        span,
                    );
                }

                let init_type = init.type_inference(config, symbols);

                if var_type.type_check(true, &init_type, &symbols.types) == false {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: symbols.types.get(var_type).format(&symbols.types),
                            found: symbols.types.get(init_type).format(&symbols.types),
                        }),
                        span,
                    );
                };
                 */
}

#[cfg(test)]
mod tests;
