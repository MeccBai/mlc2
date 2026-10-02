use super::{PackageSymbolTable, SymbolTable};
use crate::ast::{config::FileId, statement::Variable};
use std::rc::Rc;

impl SymbolTable {
    pub fn ordered_globals(&self) -> Vec<&Rc<Variable>> {
        let mut globals: Vec<_> = self.globals.iter().collect();
        globals.sort_by_key(|(name, (order, _))| (*order, *name));
        globals
            .into_iter()
            .map(|(_, (_, variable))| variable)
            .collect()
    }
}

impl PackageSymbolTable {
    /// Identity, not a source name: a local may share a global's unqualified name.
    pub fn global_owner(&self, variable: &Rc<Variable>) -> Option<FileId> {
        self.arenas.iter().find_map(|(id, symbols)| {
            let local_name = variable.name.rsplit("::").next().unwrap_or(&variable.name);
            symbols
                .globals
                .get(&variable.name)
                .or_else(|| symbols.globals.get(local_name))
                .filter(|(_, global)| Rc::ptr_eq(global, variable))
                .map(|_| id)
        })
    }
}
