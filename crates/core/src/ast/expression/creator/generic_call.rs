use crate::ast::symbols::Resolution;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::arena::GenericIndex;
use crate::ast::config::Config;
use crate::ast::expression::{Expression, FuncCall};
use crate::ast::statement::Variable;
use crate::ast::types::resolve_type;
use crate::ast::{EnumBool, SymbolTable, TypeIndex};
use crate::diagnostic::error::{CompileError, IllegalUseError, ResolveError};
use crate::parser::out::{Span, Spanned, TempExpr, TempExpr::Path, TempPath, TempType};

impl Expression {
    pub(super) fn new_associated_call(
        config: &mut Config,
        owner: Spanned<TempType>,
        name: String,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Self {
        use crate::ast::{symbol_name::SymbolName, types::CompileType};
        let Some(owner) = resolve_type(
            config,
            owner,
            symbols,
            context.map(|ctx| ctx as &dyn crate::ast::types::TypeContext),
        ) else {
            return Self::Poison(span);
        };
        let owner_name = symbols.get_type(owner).unqualified().format(symbols);
        let concrete = SymbolName::callable(Some(&owner_name), &name);
        let mut index = symbols.associated_interface(owner, &concrete);
        let args = args
            .into_iter()
            .map(|arg| Self::new(config, arg, symbols, context))
            .collect::<Vec<_>>();
        if config.is_poisoned() {
            return Self::Poison(span);
        }
        if index.is_none()
            && let CompileType::Unit(unit) = symbols.get_type(owner).unqualified()
            && let Some(application) = &unit.application
        {
            let key = SymbolName::callable(Some(&application.template_name), &name);
            if let Some(template) = symbols
                .table(application.template.file_id())
                .generics
                .interfaces
                .get_by_name(&key)
            {
                let symbol = symbols.get_interface(template, true).clone();
                if !symbol.has_self {
                    let mut bindings = HashMap::new();
                    let inferred = crate::infer::generics::infer(
                        symbol.owner,
                        owner,
                        &mut bindings,
                        &symbol.generic_map,
                        symbols,
                    )
                    .and_then(|()| {
                        for ((expected, _), arg) in symbol.params.iter().zip(&args) {
                            let actual = arg.type_inference(config, symbols);
                            if actual.is_empty() {
                                return Err(IllegalUseError::ArgumentCountMismatch);
                            }
                            crate::infer::generics::infer(
                                *expected,
                                actual,
                                &mut bindings,
                                &symbol.generic_map,
                                symbols,
                            )?;
                        }
                        Ok(())
                    });
                    if let Err(error) = inferred {
                        config.submit_error(CompileError::IllegalUse(error), span);
                        return Self::Poison(span);
                    }
                    let instance = template.instantiation(
                        config,
                        &bindings,
                        symbols,
                        context.and_then(|ctx| ctx.instantiation_actives()),
                        span,
                    );
                    if config.is_poisoned() || instance.is_empty() {
                        return Self::Poison(span);
                    }
                    index = Some(instance);
                }
            }
        }
        let Some(index) = index else {
            config.submit_error(CompileError::Resolve(ResolveError::UnknownInterface), span);
            return Self::Poison(span);
        };
        let symbol = symbols.get_interface_regular(index);
        if symbol.has_self || (!symbol.exported && index.file_id() != config.file_id()) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                span,
            );
            return Self::Poison(span);
        }
        let params = symbol.params.clone();
        if !crate::TypeInfer::new(config, symbols).check_arguments(&params, &args, span) {
            return Self::Poison(span);
        }
        Self::FuncCallE(
            FuncCall {
                pipeline: false,
                callee: None,
                func: EnumBool::True(index),
                args,
            },
            span,
        )
    }

    pub(super) fn new_generic_call(
        config: &mut Config,
        path: TempPath,
        generic_args: Vec<Spanned<TempType>>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Self {
        if crate::ast::builtins::numeric_constant(&path.segments).is_some() {
            config.submit_error(
                CompileError::IllegalUse(
                    crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                ),
                span,
            );
            return Self::Poison(span);
        }
        if Self::is_function_builtin(&path) {
            return Self::new_function_pointer(config, generic_args, args, span, symbols, context);
        }
        let params = generic_args
            .into_iter()
            .map(|arg| {
                resolve_type(
                    config,
                    arg,
                    symbols,
                    context.map(|ctx| ctx as &dyn crate::ast::types::TypeContext),
                )
                .unwrap_or(TypeIndex::empty())
            })
            .collect::<Vec<_>>();
        if config.is_poisoned() {
            return Self::Poison(span);
        }

        if path.segments.len() == 1 {
            if let Some(definition) = crate::ast::builtins::lookup(&path.segments.join("::")) {
                if params.len() != definition.type_arguments {
                    config.submit_error(
                        CompileError::IllegalUse(
                            crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                        ),
                        span,
                    );
                    return Self::Poison(span);
                }
                return Self::new_builtin_call(
                    config,
                    definition.kind,
                    params.first().copied().unwrap_or(TypeIndex::empty()),
                    args,
                    span,
                    symbols,
                    context,
                );
            }
        }

        let func = match Self::search_generic_function(config, (Path(path), span), symbols, context)
        {
            Some(index) => index,
            None => return Self::null(),
        };

        let func = match func {
            EnumBool::True(interface) => {
                let symbol = symbols.get_interface(interface, true);

                let generics_names = &symbol.generics;
                if generics_names.len() != params.len() {
                    config.submit_error(
                        CompileError::IllegalUse(
                            crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                        ),
                        span,
                    );
                    return Self::Poison(span);
                }

                let generic_map = generics_names
                    .iter()
                    .zip(params.iter())
                    .map(|(name, ty)| {
                        let index = symbol.generic_map.get(name);
                        let gen_index = match index {
                            Some(index) => index.clone(),
                            None => {
                                config.submit_error(
                                    CompileError::Resolve(ResolveError::UnknownConstraint),
                                    span,
                                );
                                GenericIndex::empty()
                            }
                        };
                        (gen_index, ty.clone())
                    })
                    .collect::<HashMap<_, _>>();

                EnumBool::True(interface.instantiation(
                    config,
                    &generic_map,
                    symbols,
                    context.and_then(|ctx| ctx.instantiation_actives()),
                    span,
                ))
            }
            EnumBool::False(func_index) => {
                let symbol = symbols.get_function(func_index, true);
                if let Some(kind) = symbol.builtin() {
                    if params.len() != 1 {
                        config.submit_error(
                            CompileError::IllegalUse(
                                crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                            ),
                            span,
                        );
                        return Self::Poison(span);
                    }
                    return Self::new_builtin_call(
                        config, kind, params[0], args, span, symbols, context,
                    );
                }
                let generics_names = &symbol.generics;
                if generics_names.len() != params.len() {
                    config.submit_error(
                        CompileError::IllegalUse(
                            crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                        ),
                        span,
                    );
                    return Self::Poison(span);
                }
                let generic_map = generics_names
                    .iter()
                    .zip(params.iter())
                    .map(|(name, ty)| {
                        let index = symbol.generic_map.get(name);
                        let gen_index = match index {
                            Some(index) => index.clone(),
                            None => {
                                config.submit_error(
                                    CompileError::Resolve(ResolveError::UnknownConstraint),
                                    span,
                                );
                                GenericIndex::empty()
                            }
                        };
                        (gen_index, ty.clone())
                    })
                    .collect::<HashMap<_, _>>();
                EnumBool::False(func_index.instantiation(
                    config,
                    &generic_map,
                    symbols,
                    context.and_then(|ctx| ctx.instantiation_actives()),
                    span,
                ))
            }
        };

        if config.is_poisoned() {
            return Self::Poison(span);
        }
        let args = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context))
            .collect::<Vec<_>>();

        if let EnumBool::False(index) = func {
            if !Self::check_function_args(config, index, &args, symbols, span) {
                return Self::Poison(span);
            }
        }
        Self::FuncCallE(
            FuncCall {
                pipeline: false,
                callee: None,
                func,
                args,
            },
            span,
        )
    }
}
