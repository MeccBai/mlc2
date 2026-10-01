use super::{
    ImportModule, TempEnum, TempFunc, TempGeneric, TempInterface, TempUnit, TempUsing, TempVar,
};
use crate::parser::out::{Spanned, TempGlobalStmt, TempModule};

/// Owns the split Temp nodes; source spans preserve diagnostic ordering.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DeferredDeclarations {
    groups: [TempModule; 8],
}

impl DeferredDeclarations {
    pub fn new(module: TempModule) -> Self {
        let mut groups: [TempModule; 8] = std::array::from_fn(|_| Vec::new());
        for item in module {
            let group = match &item.0 {
                TempGlobalStmt::Enum(_) => 0,
                TempGlobalStmt::Unit(_) => 1,
                TempGlobalStmt::Generic(_) => 2,
                TempGlobalStmt::Func(_) => 3,
                TempGlobalStmt::Interface(_) => 4,
                TempGlobalStmt::Variable(_) => 5,
                TempGlobalStmt::Using(_) => 6,
                TempGlobalStmt::Import(_) => 7,
            };
            groups[group].push(item);
        }
        Self { groups }
    }

    pub fn source_order(&self) -> Vec<&Spanned<TempGlobalStmt>> {
        let mut items: Vec<_> = self.groups.iter().flatten().collect();
        items.sort_by_key(|(_, span)| span.into_range().start);
        items
    }

    pub fn into_module(self) -> TempModule {
        self.groups.into_iter().flatten().collect()
    }
}

pub(super) fn split(
    temp_ast: TempModule,
) -> (
    Vec<TempEnum>,
    Vec<TempUnit>,
    Vec<TempFunc>,
    Vec<TempGeneric>,
    Vec<ImportModule>,
    Vec<TempVar>,
    Vec<TempUsing>,
    Vec<TempInterface>,
) {
    let mut enums = Vec::<TempEnum>::new();
    let mut units = Vec::<TempUnit>::new();
    let mut funcs = Vec::<TempFunc>::new();
    let mut generics = Vec::<TempGeneric>::new();
    let mut imports = Vec::<ImportModule>::new();
    let mut globals = Vec::<TempVar>::new();
    let mut usings = Vec::<TempUsing>::new();
    let mut interfaces = Vec::<TempInterface>::new();

    temp_ast.into_iter().for_each(|(stmt, _span)| match stmt {
        TempGlobalStmt::Unit(temp_unit) => units.push(temp_unit),
        TempGlobalStmt::Func(temp_func) => funcs.push(temp_func),
        TempGlobalStmt::Using(temp_using) => usings.push(temp_using),
        TempGlobalStmt::Generic(temp_generic) => generics.push(temp_generic),
        TempGlobalStmt::Enum(temp_enum) => enums.push(temp_enum),
        TempGlobalStmt::Import(temp_import) => imports.push(temp_import),
        TempGlobalStmt::Variable(temp_variable) => globals.push(temp_variable),
        TempGlobalStmt::Interface(interface) => interfaces.push(interface),
    });

    (
        enums, units, funcs, generics, imports, globals, usings, interfaces,
    )
}
