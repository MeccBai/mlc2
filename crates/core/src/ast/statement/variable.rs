use mlc_syntax::parser::out;

use crate::ast::{
    expression,
    symbols::{Resolution, StatementContext},
    types,
};
use crate::diagnostic::{error, warning};
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
            constant: false,
            read_count: Default::default(),
            declaration_span: (0..0).into(),
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::Poison((0..0).into())),
        })
    }
    pub fn empty() -> Rc<Self> {
        Rc::new(Variable {
            constant: false,
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
        context: Option<&mut StatementContext>,
    ) -> Rc<Self> {
        let var_span = temp_var.initializer.1;

        let ctxt = context.map(|ctxt| &*ctxt);
        let declared = match temp_var.ty.clone() {
            Some((out::TempType::InferredResource, _)) => None,
            Some(ty) => resolve_type(
                config,
                ty,
                symbols,
                ctxt.map(|context| context as &dyn types::TypeContext),
            ),
            None => None,
        };
        if config.is_poisoned() {
            return Self::poison();
        }
        let array_length = declared.and_then(|ty| match symbols.get_type(ty).unqualified() {
            types::CompileType::List(list) => Some(list.length),
            _ => None,
        });
        let previous_pointer_warning = config.suppress_pointer_warning();
        if temp_var.name.starts_with('$') {
            config.set_suppress_pointer_warning(true);
        }
        let init_val = match (array_length, temp_var.initializer.0) {
            (Some(_), out::TempExpr::Init { .. }) => {
                config.submit_error(
                    error::CompileError::IllegalUse(
                        IllegalUseError::ArrayInitializerRequiresBrackets,
                    ),
                    var_span,
                );
                return Self::poison();
            }
            (Some(length), out::TempExpr::Array(values)) => {
                if values.len() > length {
                    config.submit_error(
                        error::CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: format!("at most {length} array elements"),
                            found: format!("{} array elements", values.len()),
                        }),
                        var_span,
                    );
                    return Self::poison();
                }
                if values.len() < length {
                    let element =
                        declared.and_then(|ty| match symbols.get_type(ty).unqualified() {
                            types::CompileType::List(list) => Some(list.element_type),
                            _ => None,
                        });
                    if element.is_some_and(|ty| ty.contains_function_pointer(symbols)) {
                        config.submit_error(
                            error::CompileError::IllegalUse(
                                IllegalUseError::FunctionPointerRequiresInitializer,
                            ),
                            var_span,
                        );
                        return Self::poison();
                    }
                    config.submit_warning(
                        warning::Warning::ArrayInitializerZeroFilled {
                            supplied: values.len(),
                            length,
                        },
                        var_span,
                    );
                }
                Expression::InitListE(
                    expression::InitialList::Array {
                        ty: declared.expect("declared array"),
                        values: values
                            .into_iter()
                            .map(|value| Expression::new(config, value, symbols, ctxt))
                            .collect(),
                    },
                    var_span,
                )
            }
            (_, expr) => Expression::new(config, (expr, var_span), symbols, ctxt),
        };
        config.set_suppress_pointer_warning(previous_pointer_warning);
        if init_val.is_poisoned() {
            return Self::poison();
        }
        // Semantic constant evaluation remains mandatory for diagnostics and const.
        let init_val = if let Some(target) = declared {
            init_val.coerce_string_view(target, config, symbols, var_span)
        } else {
            init_val
        };
        let original_init = (temp_var.value_type != ValueType::Constant).then(|| init_val.clone());
        let init_val = init_val.const_fold(config, symbols);
        if let Some(ty) = declared {
            init_val.check_constant_range(ty, config, symbols, var_span);
            if config.is_poisoned() {
                return Self::poison();
            }
        }
        let inferred = init_val.type_inference(config, symbols);
        if ctxt.is_none()
            && (inferred.has_res(symbols) || declared.is_some_and(|ty| ty.has_res(symbols)))
        {
            config.submit_error(
                error::CompileError::IllegalUse(IllegalUseError::UnsupportedResourceOperation),
                var_span,
            );
            return Self::poison();
        }

        let ty = match temp_var.ty {
            Some((out::TempType::InferredResource, _)) => {
                if !inferred.has_res(symbols) {
                    config.submit_error(
                        error::CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                            expected: "resource initializer for res".into(),
                            found: if inferred.is_empty() {
                                "void".into()
                            } else {
                                inferred.format(symbols)
                            },
                        }),
                        var_span,
                    );
                    return Self::poison();
                }
                inferred
            }
            Some(_) => match declared {
                Some(ty) => {
                    if init_val.type_check(&ty, config, symbols) == false {
                        config.submit_error(
                            error::CompileError::IllegalUse(IllegalUseError::TypeMismatched {
                                expected: symbols.get_type(ty).format(symbols),
                                found: if inferred.is_empty() {
                                    "untyped initializer".into()
                                } else {
                                    symbols.get_type(inferred).format(symbols)
                                },
                            }),
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
                        error::CompileError::Resolve(error::ResolveError::MissingType),
                        var_span,
                    );
                }
                if inferred.is_resource(symbols) {
                    if !matches!(&init_val, Expression::VarValueE(_, ..)) {
                        config.submit_error(
                            error::CompileError::IllegalUse(IllegalUseError::ResourceRequiresOwner),
                            var_span,
                        );
                        return Self::poison();
                    }
                    let types::CompileType::Ref(reference) =
                        symbols.get_type(inferred).unqualified()
                    else {
                        unreachable!()
                    };
                    let reference = reference.clone().borrowed(false);
                    let name = reference.format(symbols);
                    symbols
                        .local_mut()
                        .types
                        .insert(name, types::CompileType::Ref(reference))
                } else {
                    inferred
                }
            }
        }
        .with_immutability(temp_var.value_type != ValueType::Flex, symbols);
        // Native str bindings keep their compile-time payload and cannot be rebound.
        let native_str =
            !ty.is_empty() && matches!(symbols.get_type(ty).unqualified(), types::CompileType::Str);
        let ty = if native_str {
            ty.with_immutability(true, symbols)
        } else {
            ty
        };

        if (native_str || temp_var.value_type == ValueType::Constant) && !init_val.is_const(symbols)
        {
            config.submit_error(
                error::CompileError::IllegalUse(IllegalUseError::NonConstantInitializer),
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
        if let Some(index) = &config.semantic {
            index.borrow_mut().push(crate::ast::semantic::Occurrence {
                symbol: None,
                span: temp_var.name_span,
                ty: ty.format(symbols),
                kind: "variable",
                definition: Some((config.file_id(), temp_var.name_span)),
                parameters: vec![],
            });
        }
        Rc::new(Self {
            constant: (native_str && ctxt.is_some()) || temp_var.value_type == ValueType::Constant,
            read_count: Default::default(),
            declaration_span: temp_var.name_span,
            name: temp_var.name,
            var_type: ty,
            init_val: Box::new(
                original_init
                    .filter(|_| !native_str)
                    .unwrap_or(init_val)
                    .normalize_initializer(ty, symbols),
            ),
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
