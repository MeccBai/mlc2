use super::{ExportSymbol, ExportTable, PackageSymbolTable};
use crate::ast::config::FileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcatError {
    DuplicateSymbol(String),
    UnknownFile(FileId),
    InvalidExport(String),
}

impl PackageSymbolTable {
    /// Attach an export index to arenas already owned by this package.
    /// Repeating publication of the same source symbol is allowed.
    pub fn concat(&mut self, table: ExportTable) -> Result<(), ConcatError> {
        let id = table.file_id;
        if !self.configs.contains_key(&id) {
            return Err(ConcatError::UnknownFile(id));
        }
        let file = self.file(id).ok_or(ConcatError::UnknownFile(id))?;
        for (name, symbol) in table.searchable.iter().chain(&table.inner) {
            let valid = match *symbol {
                ExportSymbol::Type(index) => file.types.contains(index),
                ExportSymbol::GenericUnit(index) => file.generics.units.contains(index),
                ExportSymbol::GenericRequire(index) => file.generics.requires.contains(index),
                ExportSymbol::Function { index, generic } => {
                    if generic {
                        file.generics.functions.contains(index)
                    } else {
                        file.functions.contains(index)
                    }
                }
                ExportSymbol::Interface { index, generic } => {
                    if generic {
                        file.generics.interfaces.contains(index)
                    } else {
                        file.interfaces.contains(index)
                    }
                }
            };
            if !valid {
                return Err(ConcatError::InvalidExport(name.clone()));
            }
            if self
                .searchable
                .get(name)
                .is_some_and(|existing| existing != symbol)
            {
                return Err(ConcatError::DuplicateSymbol(name.clone()));
            }
        }
        self.searchable.retain(|_, symbol| symbol.file_id() != id);
        self.searchable.extend(table.searchable.clone());
        self.exports.insert(id, table);
        Ok(())
    }
}

impl ExportSymbol {
    pub fn file_id(self) -> FileId {
        match self {
            Self::Type(index) => index.file_id(),
            Self::GenericUnit(index) => index.file_id(),
            Self::GenericRequire(index) => index.file_id(),
            Self::Function { index, .. } => index.file_id(),
            Self::Interface { index, .. } => index.file_id(),
        }
    }
}

#[cfg(test)]
mod tests;
