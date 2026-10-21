use super::TypeInfer;
use crate::ast::{
    TypeIndex,
    expression::{Expression, InitialList},
    types::{CompileType, base_type::DataType},
};
impl TypeInfer<'_> {
    /// Context-sensitive checking with exactly one diagnostic submission point.
    pub fn check(
        &mut self,
        expression: &Expression,
        target: TypeIndex,
        mode: super::CheckMode,
        span: crate::parser::out::Span,
    ) -> bool {
        if self.config.is_poisoned() || expression.is_poisoned() {
            return false;
        }
        if target.is_empty() {
            self.config.submit_error(
                crate::diagnostic::error::CompileError::Resolve(
                    crate::diagnostic::error::ResolveError::MissingType,
                ),
                span,
            );
            return false;
        }
        if mode != super::CheckMode::Argument {
            self.check_constant_range(expression, target, span);
            if self.config.is_poisoned() {
                return false;
            }
        }
        let accepted = if mode == super::CheckMode::Argument {
            let actual = self.type_of(expression);
            !actual.is_empty() && target.type_check(false, &actual, self.symbols)
        } else if mode == super::CheckMode::Initialization {
            let constant = self.constant(expression.clone());
            self.compatible(&constant, target)
        } else {
            self.compatible(expression, target)
        };
        if !accepted && !self.config.is_poisoned() {
            let actual = self.type_of(expression);
            self.config.submit_error(
                crate::diagnostic::error::CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::TypeMismatched {
                        expected: target.format(self.symbols),
                        found: if actual.is_empty() {
                            "void".into()
                        } else {
                            actual.format(self.symbols)
                        },
                    },
                ),
                span,
            );
        }
        accepted
    }

    pub fn check_arguments(
        &mut self,
        parameters: &[(TypeIndex, String)],
        arguments: &[Expression],
        span: crate::parser::out::Span,
    ) -> bool {
        if self.config.is_poisoned() || arguments.iter().any(Expression::is_poisoned) {
            return false;
        }
        let variadic = parameters.last().is_some_and(|(_, name)| name == "...");
        let fixed = parameters.len() - usize::from(variadic);
        if arguments.len() < fixed || (!variadic && arguments.len() != fixed) {
            self.config.submit_error(
                crate::diagnostic::error::CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::ArgumentCountMismatch,
                ),
                span,
            );
            return false;
        }
        arguments
            .iter()
            .zip(parameters.iter().take(fixed))
            .all(|(argument, (expected, _))| {
                self.check(argument, *expected, super::CheckMode::Argument, span)
            })
    }

    pub fn compatible(&mut self, expression: &Expression, target: TypeIndex) -> bool {
        let config = &mut *self.config;
        let symbols = &mut *self.symbols;
        let target = &target;
        if config.is_poisoned() || expression.is_poisoned() {
            return false;
        }
        if let Some(accepted) = expression.numeric_initialization(*target, config, symbols) {
            return accepted;
        }
        match expression {
            Expression::Poison(..) => false,
            Expression::VarValueE(var, ..) => target.type_check(true, &var.var_type, symbols),
            Expression::ConstValueE(const_value, ..) => {
                target.type_check(true, &const_value.ty, symbols)
            }
            Expression::UnaryExprE(unary, ..) => unary
                .type_inference(config, symbols)
                .type_check(true, &target, symbols),
            Expression::InitListE(init_list, ..) => match init_list {
                InitialList::List { onwer, values } => match onwer {
                    Some(ty) => ty == target,
                    None => {
                        let check_list_types = match symbols.get_type(target.clone()).unqualified()
                        {
                            CompileType::Unit(unit) => {
                                if let Some(variant) = &unit.variant {
                                    let candidates = variant.candidates.clone();
                                    if values.len() != 1 {
                                        return false;
                                    }
                                    let actual = values[0].type_inference(config, symbols);
                                    return candidates.iter().any(|candidate| {
                                        !actual.is_empty()
                                            && symbols
                                                .get_type(*candidate)
                                                .unqualified()
                                                .format(symbols)
                                                == symbols
                                                    .get_type(actual)
                                                    .unqualified()
                                                    .format(symbols)
                                    });
                                }
                                let members: Vec<_> = unit
                                    .members
                                    .iter()
                                    .map(|member| member.member_type.clone())
                                    .collect();
                                Some(members)
                            }
                            _ => None,
                        };
                        match check_list_types {
                            Some(list_types) => {
                                if list_types.len() != values.len() {
                                    return false;
                                }
                                values
                                    .iter()
                                    .zip(list_types.iter())
                                    .all(|(value, ty)| value.type_check(ty, config, symbols))
                            }
                            None => false,
                        }
                    }
                },
                InitialList::Array { ty, values } => {
                    if !target.type_check(true, ty, symbols) {
                        return false;
                    }
                    let element_type = match symbols.get_type(*ty).unqualified() {
                        CompileType::List(list_type) => {
                            if values.len() > list_type.length {
                                return false;
                            }
                            list_type.element_type
                        }
                        _ => return false,
                    };
                    values
                        .iter()
                        .all(|value| value.type_check(&element_type, config, symbols))
                }
                InitialList::String { value } => {
                    let string_type = symbols.get_base(DataType::Integer, 8, true);
                    let target = symbols.get_type(target.clone()).unqualified();

                    if matches!(target, CompileType::Str) {
                        return true;
                    }
                    let list = match target {
                        CompileType::List(list_type) => list_type,
                        _ => return false,
                    };

                    if list.length < value.len() {
                        return false;
                    }

                    list.element_type.type_check(false, &string_type, symbols)
                }
            },
            Expression::CompositeE(composite, ..) => composite
                .type_inference(config, symbols)
                .type_check(false, &target, symbols),
            Expression::FuncCallE(func_call, ..) => func_call
                .type_inference(symbols)
                .type_check(false, &target, symbols),
        }
    }
}
