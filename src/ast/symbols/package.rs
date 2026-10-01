use super::{ExportSymbol, ExportTable, SymbolTable};
use crate::ast::{
    arena::{ArenaStore, FuncIndex, GenericIndex, InterfaceIndex, TypeIndex},
    config::{Config, FileId},
    function::{FuncSymbol, InterfaceSymbol},
    generic::{GenericRequire, UnitIndex},
    types::{CompileType, UnitType},
};

#[derive(Debug)]
pub struct PackageSymbolTable {
    pub(super) arenas: ArenaStore<SymbolTable>,
    pub(super) names: std::collections::HashMap<String, FileId>,
    pub(super) configs: std::collections::HashMap<FileId, Config>,
    pub(super) exports: std::collections::HashMap<FileId, ExportTable>,
    pub(super) searchable: std::collections::HashMap<String, ExportSymbol>,
}

impl Default for PackageSymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PackageSymbolTable {
    pub fn new() -> Self {
        Self {
            arenas: ArenaStore::new(),
            names: std::collections::HashMap::new(),
            configs: std::collections::HashMap::new(),
            exports: std::collections::HashMap::new(),
            searchable: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, id: FileId) -> Result<(), FileId> {
        self.arenas.register(id, SymbolTable::for_file(id))
    }

    pub fn file(&self, id: FileId) -> Option<&SymbolTable> {
        self.arenas.get(id)
    }
    pub fn file_mut(&mut self, id: FileId) -> Option<&mut SymbolTable> {
        self.arenas.get_mut(id)
    }

    /// Check the entire batch before reserving any name across files.
    pub(crate) fn claim_names(
        &mut self,
        id: FileId,
    ) -> Result<(), (String, crate::parser::out::Span)> {
        let file = self.arenas.get(id).expect("registered file");
        let mut declarations: Vec<_> = file.declaration_spans.iter().collect();
        declarations.sort_by_key(|(_, span)| span.into_range().start);
        for (name, span) in &declarations {
            if self.names.get(*name).is_some_and(|owner| *owner != id) {
                return Err(((*name).clone(), **span));
            }
        }
        self.names
            .extend(declarations.into_iter().map(|(name, _)| (name.clone(), id)));
        Ok(())
    }

    pub fn declared_file(&self, name: &str) -> Option<FileId> {
        self.names.get(name).copied()
    }

    pub fn config(&self, id: FileId) -> Option<&Config> {
        self.configs.get(&id)
    }

    pub fn lookup(&self, name: &str, caller: FileId) -> Option<ExportSymbol> {
        self.searchable.get(name).copied().or_else(|| {
            self.exports
                .get(&caller)
                .and_then(|table| table.inner.get(name).copied())
        })
    }

    pub(crate) fn update_config(&mut self, config: &Config) {
        self.configs.insert(config.file_id(), config.clone());
    }

    pub fn get_type(&self, index: TypeIndex) -> &CompileType {
        self.file(index.file_id())
            .expect("registered type arena")
            .types
            .get(index)
    }
    pub fn get_type_mut(&mut self, index: TypeIndex) -> &mut CompileType {
        self.file_mut(index.file_id())
            .expect("registered type arena")
            .types
            .get_mut(index)
    }
    pub fn get_function(&self, index: FuncIndex, generic: bool) -> &FuncSymbol {
        let file = self
            .file(index.file_id())
            .expect("registered function arena");
        if generic {
            file.generics.functions.get(index)
        } else {
            file.functions.get(index)
        }
    }
    pub fn get_interface(&self, index: InterfaceIndex, generic: bool) -> &InterfaceSymbol {
        let file = self
            .file(index.file_id())
            .expect("registered interface arena");
        if generic {
            file.generics.interfaces.get(index)
        } else {
            file.interfaces.get(index)
        }
    }
    pub fn get_generic(&self, index: GenericIndex) -> &GenericRequire {
        self.file(index.file_id())
            .expect("registered generic arena")
            .generics
            .requires
            .get(index)
    }

    pub fn get_generic_unit(&self, index: UnitIndex) -> &UnitType {
        self.file(index.file_id())
            .expect("registered unit arena")
            .generics
            .units
            .get(index)
    }

    #[cfg(test)]
    pub(crate) fn take_file(&mut self, id: FileId) -> SymbolTable {
        self.arenas.remove(id).expect("registered test file")
    }
}
