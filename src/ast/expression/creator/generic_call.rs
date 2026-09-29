use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::arena::GenericIndex;
use crate::ast::config::Config;
use crate::ast::expression::{Expression, FuncCall};
use crate::ast::statement::Variable;
use crate::ast::types::resolve_type;
use crate::ast::{EnumBool, SymbolTable, TypeIndex};
use crate::error::{CompileError, ResolveError};
use crate::parser::out::{Span, Spanned, TempExpr, TempExpr::Path, TempPath, TempType};

impl Expression {
    pub(super) fn new_generic_call(
        config: &mut Config,
        path: TempPath,
        generic_args: Vec<Spanned<TempType>>,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> Self {
        let params = generic_args
            .into_iter()
            .map(|arg| resolve_type(config, arg, symbols).unwrap_or(TypeIndex::empty()))
            .collect::<Vec<_>>();

        let func = match Self::search_generic_function(config, (Path(path), span), symbols, context)
        {
            Some(index) => index,
            None => return Self::null(),
        };

        let func = match func {
            EnumBool::True(interface) => {
                let symbol = symbols.generics.interfaces.get(interface);

                let generics_names = &symbol.generics;

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

                EnumBool::True(interface.instantiation(config, &generic_map, symbols, span))
            }
            EnumBool::False(func_index) => {
                let symbol = symbols.generics.functions.get(func_index);
                let generics_names = &symbol.generics;
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
                EnumBool::False(func_index.instantiation(config, &generic_map, symbols, span))
            }
        };

        let args = args
            .into_iter()
            .map(|arg| Expression::new(config, arg, symbols, context))
            .collect::<Vec<_>>();

        Self::FuncCallE(FuncCall { func, args })
    }
}
