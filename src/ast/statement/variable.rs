use std::{collections::HashMap, rc::Rc};

use crate::{
    ast::{
        SymbolTable, arena::TypeIndex, config::Config, expression::Expression, statement::Variable,
        types::resolve_type,
    },
    error::IllegalUseError,
    parser::out::TempVar,
};

impl Variable {
    pub fn empty() -> Rc<Self> {
        Rc::new(Variable {
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::null()),
            immutable: false,
        })
    }

    pub fn new(
        config: &mut Config,
        temp_var: TempVar,
        symbols: &mut SymbolTable,
        context: Option<&mut HashMap<String, Rc<Variable>>>,
    ) -> Rc<Self> {
        let var_span = temp_var.initializer.1;

        let ctxt = context.map(|ctxt| &*ctxt);
        let init_val = Expression::new(config, temp_var.initializer, symbols, ctxt);
        let inferred = init_val.type_inference(config, symbols);

        let ty = match temp_var.ty {
            Some(ty) => match resolve_type(config, ty, symbols) {
                Some(ty) => {
                    if ty.type_check(true, &inferred, &symbols.types) == false {
                        config.submit_error(
                            crate::error::CompileError::IllegalUse(
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
                        crate::error::CompileError::Resolve(
                            crate::error::ResolveError::MissingType,
                        ),
                        var_span,
                    );
                }
                inferred
            }
        };

        Rc::new(Self {
            name: temp_var.name,
            var_type: ty,
            init_val: Box::new(init_val),
            immutable: temp_var.constant,
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
