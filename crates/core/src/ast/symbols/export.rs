use crate::ast::arena::{ArenaIndex, InterfaceIndex};
use crate::ast::config::FileId;
use crate::ast::symbols::PackageSymbolTable;
use crate::ast::{Config, FuncIndex, GenericIndex, TypeIndex, UnitType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSymbol {
    Type(TypeIndex),
    GenericUnit(ArenaIndex<UnitType>),
    GenericRequire(GenericIndex),
    Function {
        index: FuncIndex,
        generic: bool,
    },
    Interface {
        index: InterfaceIndex,
        generic: bool,
    },
}

/// References into the package, not a copy of independently indexed entities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTable {
    pub file_id: FileId,
    pub searchable: HashMap<String, ExportSymbol>,
    pub inner: HashMap<String, ExportSymbol>,
}

impl ExportTable {
    pub fn empty(file_id: FileId) -> Self {
        Self {
            file_id,
            searchable: HashMap::new(),
            inner: HashMap::new(),
        }
    }

    /// Private declarations are only available in their defining file.
    pub fn lookup(&self, name: &str, caller: FileId) -> Option<ExportSymbol> {
        self.searchable.get(name).copied().or_else(|| {
            (caller == self.file_id)
                .then(|| self.inner.get(name).copied())
                .flatten()
        })
    }

    fn insert(&mut self, name: String, symbol: ExportSymbol, exported: bool) {
        if exported {
            self.searchable.insert(name, symbol);
        } else {
            self.inner.insert(name, symbol);
        }
    }
}

pub(crate) fn collect(
    config: &Config,
    package: &PackageSymbolTable,
    exported_types: &HashSet<String>,
) -> ExportTable {
    let file = package.file(config.file_id()).expect("prepared file");
    let mut out = ExportTable::empty(config.file_id());
    for (name, index, _) in file.types.entries() {
        out.insert(
            name.clone(),
            ExportSymbol::Type(index),
            exported_types.contains(name),
        );
    }
    for (name, index, unit) in file.generics.units.entries() {
        out.insert(
            name.clone(),
            ExportSymbol::GenericUnit(index),
            unit.exported,
        );
    }
    for (name, index, requirement) in file.generics.requires.entries() {
        out.insert(
            name.clone(),
            ExportSymbol::GenericRequire(index),
            requirement.exported,
        );
    }
    for (generic, arena) in [(false, &file.functions), (true, &file.generics.functions)] {
        for (name, index, symbol) in arena.entries() {
            out.insert(
                config.symbol_name(name),
                ExportSymbol::Function { index, generic },
                symbol.exported,
            );
        }
    }
    for (generic, arena) in [(false, &file.interfaces), (true, &file.generics.interfaces)] {
        for (name, index, symbol) in arena.entries() {
            out.insert(
                name.clone(),
                ExportSymbol::Interface { index, generic },
                symbol.exported,
            );
        }
    }
    out
}
