use crate::ast::symbols::Resolution;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::arena::GenericIndex;
use crate::ast::config::Config;
use crate::ast::expression::{Expression, FuncCall};
use crate::ast::statement::Variable;
use crate::ast::types::resolve_type;
use crate::ast::{EnumBool, SymbolTable, TypeIndex};
use crate::diagnostic::error::{CompileError, ResolveError};
use crate::parser::out::{Span, Spanned, TempExpr, TempExpr::Path, TempPath, TempType};

impl Expression {
    pub(super) fn new_generic_call(
        config: &mut Config,
        path: TempPath,
        generic_args: Vec<Spanned<TempType>>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Self {
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
            return Self::Poison;
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
                    return Self::Poison;
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
                let generics_names = &symbol.generics;
                if generics_names.len() != params.len() {
                    config.submit_error(
                        CompileError::IllegalUse(
                            crate::diagnostic::error::IllegalUseError::GenericCountMismatch,
                        ),
                        span,
                    );
                    return Self::Poison;
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
            return Self::Poison;
        }
        let args = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context))
            .collect::<Vec<_>>();

        if let EnumBool::False(index) = func {
            if !Self::check_function_args(config, index, &args, symbols, span) {
                return Self::Poison;
            }
        }
        Self::FuncCallE(FuncCall { func, args })
    }
}
