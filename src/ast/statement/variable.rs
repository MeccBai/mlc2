use std::rc::Rc;

use crate::{
    ast::{
        SymbolTable, arena::TypeIndex, config::Config, expression::Expression, statement::Variable,
        types::resolve_type,
    },
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

    pub fn new(config: &mut Config, temp_var: TempVar, symbols: &mut SymbolTable) -> Rc<Self> {
        let initializer_span = temp_var.initializer.1;
        let init_val = Expression::new(config, temp_var.initializer, symbols, None);

        let ty = match temp_var.ty {
            Some(ty) => match resolve_type(config, ty, symbols) {
                Some(ty) => ty,
                None => TypeIndex::empty(),
            },
            None => {
                let inferred = init_val.type_inference(config, symbols);
                if inferred.is_empty() {
                    config.submit_error(
                        crate::error::CompileError::Resolve(
                            crate::error::ResolveError::MissingType,
                        ),
                        initializer_span,
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
}
