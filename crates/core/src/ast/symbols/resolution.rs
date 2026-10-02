//! Search permission and arena access are deliberately separate.
use super::{
    ExportSymbol, PATH_SEARCH_ORDER, PackageSymbolTable, PathSymbol, PathSymbolKind,
    StatementContext, SymbolTable,
};
use crate::ast::{
    TypeIndex,
    arena::{FuncIndex, GenericIndex, InterfaceIndex},
    config::{Config, FileId},
    function::{FuncSymbol, InterfaceSymbol},
    generic::{GenericRequire, UnitIndex},
    types::{CompileType, TypeLookup, TypeStorage, UnitType},
};
use crate::parser::out::{TempPath, TempScope};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveScope {
    pub current: FileId,
    pub imports: HashSet<FileId>,
}
impl ResolveScope {
    pub fn new(current: FileId, imports: impl IntoIterator<Item = FileId>) -> Self {
        Self {
            current,
            imports: imports.into_iter().collect(),
        }
    }
}

/// The same parser works with a standalone table or a package-backed view.
pub trait Resolution: TypeStorage {
    fn local(&self) -> &SymbolTable;
    fn local_mut(&mut self) -> &mut SymbolTable;
    fn table(&self, file: FileId) -> &SymbolTable;
    fn lookup_export(&self, _name: &str) -> Option<ExportSymbol> {
        None
    }
    fn local_type(&self, name: &str) -> Option<TypeIndex> {
        self.local().types.get_by_name(&name.to_owned())
    }
    fn file_config(&self, _file: FileId) -> Option<Config> {
        None
    }
    fn select_file(&mut self, _file: FileId) -> Option<ResolveScope> {
        None
    }
    fn restore_scope(&mut self, _scope: ResolveScope) {}

    fn get_function(&self, index: FuncIndex, generic: bool) -> &FuncSymbol {
        let file = self.table(index.file_id());
        if generic {
            file.generics.functions.get(index)
        } else {
            file.functions.get(index)
        }
    }
    fn get_function_regular(&self, index: FuncIndex) -> &FuncSymbol {
        self.get_function(index, false)
    }
    fn get_interface_regular(&self, index: InterfaceIndex) -> &InterfaceSymbol {
        self.get_interface(index, false)
    }
    fn get_interface(&self, index: InterfaceIndex, generic: bool) -> &InterfaceSymbol {
        let file = self.table(index.file_id());
        if generic {
            file.generics.interfaces.get(index)
        } else {
            file.interfaces.get(index)
        }
    }
    fn get_generic_unit(&self, index: UnitIndex) -> &UnitType {
        self.table(index.file_id()).generics.units.get(index)
    }
    fn get_generic(&self, index: GenericIndex) -> &GenericRequire {
        self.table(index.file_id()).generics.requires.get(index)
    }
    fn function_template(&self, index: FuncIndex) -> Option<TempScope> {
        self.table(index.file_id())
            .generics
            .function_templates
            .get(&index)
            .cloned()
    }
    fn interface_template(&self, index: InterfaceIndex) -> Option<TempScope> {
        self.table(index.file_id())
            .generics
            .interface_templates
            .get(&index)
            .cloned()
    }
    fn get_base(
        &self,
        data: crate::ast::types::base_type::DataType,
        bits: usize,
        signed: bool,
    ) -> TypeIndex {
        self.local().get_base(data, bits, signed)
    }
    fn using_target(&self, config: &Config, name: &str) -> Option<String> {
        self.local().using_target(config, name)
    }

    fn resolve_path(
        &self,
        config: &Config,
        path: &TempPath,
        context: Option<&StatementContext>,
    ) -> Option<PathSymbol> {
        for kind in PATH_SEARCH_ORDER {
            if kind == PathSymbolKind::Using {
                if let Some(target) = self.using_target(config, &path.segments.join("::")) {
                    return self.resolve_path(
                        config,
                        &TempPath {
                            segments: target.split("::").map(str::to_owned).collect(),
                        },
                        None,
                    );
                }
                continue;
            }
            if kind != PathSymbolKind::EnumValue {
                if let Some(local) = self.local().resolve_kind(config, path, context, kind) {
                    return Some(local);
                }
            }
            if let Some(foreign) = self.export_path(config, path, kind) {
                return Some(foreign);
            }
        }
        None
    }

    fn export_path(
        &self,
        config: &Config,
        path: &TempPath,
        kind: PathSymbolKind,
    ) -> Option<PathSymbol> {
        if kind == PathSymbolKind::EnumValue {
            let (variant, owner) = path.segments.split_last()?;
            let name = owner.join("::");
            let index = self
                .local_type(&config.symbol_name(&name))
                .or_else(|| self.local_type(&name))
                .or_else(|| match self.named_export(config, &name) {
                    Some(ExportSymbol::Type(index)) => Some(index),
                    _ => None,
                })?;
            let CompileType::Enum(enumeration) = self.get_type(index).unqualified() else {
                return None;
            };
            return enumeration
                .variants
                .iter()
                .position(|name| name == variant)
                .map(|value| {
                    PathSymbol::EnumValue(crate::ast::expression::EnumValue {
                        enum_type: index,
                        value,
                    })
                });
        }
        match (kind, self.named_export(config, &path.segments.join("::"))?) {
            (PathSymbolKind::Function, ExportSymbol::Function { index, generic }) => {
                Some(PathSymbol::Function { index, generic })
            }
            (PathSymbolKind::Interface, ExportSymbol::Interface { index, generic }) => {
                Some(PathSymbol::Interface { index, generic })
            }
            _ => None,
        }
    }

    fn named_export(&self, config: &Config, name: &str) -> Option<ExportSymbol> {
        self.lookup_export(name)
            .or_else(|| self.lookup_export(&config.symbol_name(name)))
    }

    fn find_interface(&self, name: &str) -> Option<InterfaceIndex> {
        self.local()
            .interfaces
            .get_by_name(&name.to_owned())
            .or_else(|| match self.lookup_export(name) {
                Some(ExportSymbol::Interface {
                    index,
                    generic: false,
                }) => Some(index),
                _ => None,
            })
    }

    /// A known value's associated metadata is not a free name lookup.
    /// The use site still enforces public/self access rules.
    fn associated_interface(&self, owner: TypeIndex, name: &str) -> Option<InterfaceIndex> {
        self.find_interface(name).or_else(|| {
            self.table(owner.file_id())
                .interfaces
                .get_by_name(&name.to_owned())
        })
    }
}

impl TypeStorage for SymbolTable {
    fn insert_type(&mut self, name: String, ty: CompileType) -> TypeIndex {
        self.types.insert(name, ty)
    }
}
impl Resolution for SymbolTable {
    fn local(&self) -> &SymbolTable {
        self
    }
    fn local_mut(&mut self) -> &mut SymbolTable {
        self
    }
    fn table(&self, file: FileId) -> &SymbolTable {
        assert_eq!(file, self.types.file_id());
        self
    }
}

pub struct ResolveContext<'a> {
    package: &'a mut PackageSymbolTable,
    scope: ResolveScope,
}
impl<'a> ResolveContext<'a> {
    pub fn new(package: &'a mut PackageSymbolTable, scope: ResolveScope) -> Self {
        assert!(
            package.file(scope.current).is_some(),
            "registered current file"
        );
        Self { package, scope }
    }
    pub fn scope(&self) -> &ResolveScope {
        &self.scope
    }
}
impl TypeLookup for ResolveContext<'_> {
    fn get_type(&self, index: TypeIndex) -> &CompileType {
        self.package.get_type(index)
    }
}
impl TypeStorage for ResolveContext<'_> {
    fn insert_type(&mut self, name: String, ty: CompileType) -> TypeIndex {
        self.local_mut().types.insert(name, ty)
    }
}
impl Resolution for ResolveContext<'_> {
    fn associated_interface(&self, owner: TypeIndex, name: &str) -> Option<InterfaceIndex> {
        let type_name = self.get_type(owner).unqualified().format(self);
        let file = self
            .package
            .declared_file(&type_name)
            .unwrap_or(owner.file_id());
        self.find_interface(name)
            .or_else(|| self.table(file).interfaces.get_by_name(&name.to_owned()))
            .or_else(|| match self.package.searchable.get(name) {
                Some(ExportSymbol::Interface {
                    index,
                    generic: false,
                }) if self.get_interface(*index, false).public => Some(*index),
                _ => None,
            })
    }
    fn local(&self) -> &SymbolTable {
        self.package
            .file(self.scope.current)
            .expect("registered current file")
    }
    fn local_mut(&mut self) -> &mut SymbolTable {
        self.package
            .file_mut(self.scope.current)
            .expect("registered current file")
    }
    fn table(&self, file: FileId) -> &SymbolTable {
        self.package.file(file).expect("registered indexed file")
    }
    fn lookup_export(&self, name: &str) -> Option<ExportSymbol> {
        self.package.lookup_in(name, &self.scope)
    }
    fn local_type(&self, name: &str) -> Option<TypeIndex> {
        // Interning a qualified value must not accidentally expose the private
        // source type's name in a different file's declaration search space.
        self.package
            .local_type_name(self.scope.current, name)
            .then(|| self.local().types.get_by_name(&name.to_owned()))
            .flatten()
    }
    fn file_config(&self, file: FileId) -> Option<Config> {
        self.package.config(file).cloned()
    }
    fn select_file(&mut self, file: FileId) -> Option<ResolveScope> {
        let scope = self.package.resolve_scope(file);
        Some(std::mem::replace(&mut self.scope, scope))
    }
    fn restore_scope(&mut self, scope: ResolveScope) {
        self.scope = scope;
    }
}

impl<T: Resolution + ?Sized> Resolution for &mut T {
    fn associated_interface(&self, owner: TypeIndex, name: &str) -> Option<InterfaceIndex> {
        (**self).associated_interface(owner, name)
    }
    fn local(&self) -> &SymbolTable {
        (**self).local()
    }
    fn local_mut(&mut self) -> &mut SymbolTable {
        (**self).local_mut()
    }
    fn table(&self, file: FileId) -> &SymbolTable {
        (**self).table(file)
    }
    fn lookup_export(&self, name: &str) -> Option<ExportSymbol> {
        (**self).lookup_export(name)
    }
    fn local_type(&self, name: &str) -> Option<TypeIndex> {
        (**self).local_type(name)
    }
    fn file_config(&self, file: FileId) -> Option<Config> {
        (**self).file_config(file)
    }
    fn select_file(&mut self, file: FileId) -> Option<ResolveScope> {
        (**self).select_file(file)
    }
    fn restore_scope(&mut self, scope: ResolveScope) {
        (**self).restore_scope(scope)
    }
}
