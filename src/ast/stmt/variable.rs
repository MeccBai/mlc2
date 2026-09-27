use crate::{
    ast::{
        SymbolTable, arena::TypeIndex, config::Config, expr::Expression, stmt::Variable,
        types::resolve_type,
    },
    parser::out::TempVar,
};

impl Variable {
    pub fn empty() -> Self {
        Variable {
            name: String::new(),
            var_type: TypeIndex::empty(),
            init_val: Box::new(Expression::null()),
            immutable: false,
        }
    }

    pub fn new(config: &mut Config, temp_var: TempVar, symbols: &mut SymbolTable) -> Self {
        let init_val = Expression::new(config, temp_var.initializer, symbols);

        let ty = match temp_var.ty {
            Some(ty) => match resolve_type(config, ty, symbols) {
                Some(ty) => ty,
                None => TypeIndex::empty(),
            },
            None => init_val.type_inference(config, symbols),
        };

        Self {
            name: temp_var.name,
            var_type: ty,
            init_val: Box::new(init_val),
            immutable: temp_var.constant,
        }
    }
}
