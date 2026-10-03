use crate::ast::symbols::Resolution;
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
            read_count: Default::default(),
            declaration_span: (0..0).into(),
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::Poison),
        })
    }
    pub fn empty() -> Rc<Self> {
        Rc::new(Variable {
            read_count: Default::default(),
            declaration_span: (0..0).into(),
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::null()),
        })
    }

    pub fn new(
        config: &mut Config,
        temp_var: TempVar,
        symbols: &mut dyn Resolution,
        context: Option<&mut crate::ast::symbols::StatementContext>,
    ) -> Rc<Self> {
        let var_span = temp_var.initializer.1;

        let ctxt = context.map(|ctxt| &*ctxt);
        let declared = match temp_var.ty.clone() {
            Some(ty) => resolve_type(
                config,
                ty,
                symbols,
                ctxt.map(|context| context as &dyn crate::ast::types::TypeContext),
            ),
            None => None,
        };
        if config.is_poisoned() {
            return Self::poison();
        }
        let array_length = declared.and_then(|ty| match symbols.get_type(ty).unqualified() {
            crate::ast::types::CompileType::List(list) => Some(list.length),
            _ => None,
        });
        let init_val = match (array_length, temp_var.initializer.0) {
            (Some(_), crate::parser::out::TempExpr::Init { .. }) => {
                config.submit_error(
                    crate::diagnostic::error::CompileError::IllegalUse(
                        IllegalUseError::ArrayInitializerRequiresBrackets,
                    ),
                    var_span,
                );
                return Self::poison();
            }
            (Some(length), crate::parser::out::TempExpr::Array(values)) => {
                if values.len() > length {
                    config.submit_error(
                        crate::diagnostic::error::CompileError::IllegalUse(
                            IllegalUseError::TypeMismatched {
                                expected: format!("at most {length} array elements"),
                                found: format!("{} array elements", values.len()),
                            },
                        ),
                        var_span,
                    );
                    return Self::poison();
                }
                if values.len() < length {
                    config.submit_warning(
                        crate::diagnostic::warning::Warning::ArrayInitializerZeroFilled {
                            supplied: values.len(),
                            length,
                        },
                        var_span,
                    );
                }
                Expression::InitListE(crate::ast::expression::InitialList::Array {
                    ty: declared.expect("declared array"),
                    values: values
                        .into_iter()
                        .map(|value| Expression::new(config, value, symbols, ctxt))
                        .collect(),
                })
            }
            (_, expr) => Expression::new(config, (expr, var_span), symbols, ctxt),
        };
        if init_val.is_poisoned() {
            return Self::poison();
        }
        let init_val = init_val.const_fold(config, symbols);
        if let Some(ty) = declared {
            init_val.check_constant_range(ty, config, symbols, var_span);
            if config.is_poisoned() {
                return Self::poison();
            }
        }
        let inferred = init_val.type_inference(config, symbols);

        let ty = match temp_var.ty {
            Some(_) => match declared {
                Some(ty) => {
                    if init_val.type_check(&ty, config, symbols) == false {
                        config.submit_error(
                            crate::diagnostic::error::CompileError::IllegalUse(
                                IllegalUseError::TypeMismatched {
                                    expected: symbols.get_type(ty).format(symbols),
                                    found: if inferred.is_empty() {
                                        "untyped initializer".into()
                                    } else {
                                        symbols.get_type(inferred).format(symbols)
                                    },
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
        .into_value_type(temp_var.value_type, symbols);

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
        init_val.check_constant_range(ty, config, symbols, var_span);
        if config.is_poisoned() {
            return Self::poison();
        }
        Rc::new(Self {
            read_count: Default::default(),
            declaration_span: temp_var.name_span,
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

                if var_type.type_check(true, &init_type, symbols) == false {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: symbols.get_type(var_type).format(symbols),
                            found: symbols.get_type(init_type).format(symbols),
                        }),
                        span,
                    );
                };
                 */
}

#[cfg(test)]
mod tests;
