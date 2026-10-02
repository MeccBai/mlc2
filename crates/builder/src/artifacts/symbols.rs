use mlc_syntax::parser::out::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Symbols {
    pub units: Vec<TempUnit>,
    pub enums: Vec<TempEnum>,
    pub functions: Vec<TempFuncSymbol>,
    pub interfaces: Vec<TempInterfaceSymbol>,
    pub generics: Vec<TempGeneric>,
    pub inner: TempModule,
}

impl Symbols {
    pub fn from_module(module: &TempModule) -> Self {
        let mut symbols = Self::default();
        for (item, span) in module {
            match item {
                TempGlobalStmt::Unit(v) if exported(v.visibility) => symbols.units.push(v.clone()),
                TempGlobalStmt::Enum(v) if exported(v.visibility) => symbols.enums.push(v.clone()),
                TempGlobalStmt::Func(v) if exported(v.symbol.visibility) => {
                    symbols.functions.push(v.symbol.clone())
                }
                TempGlobalStmt::Interface(v) if exported(v.symbol.visibility) => {
                    symbols.interfaces.push(v.symbol.clone())
                }
                TempGlobalStmt::Generic(v) if exported(v.visibility) => {
                    symbols.generics.push(v.clone())
                }
                TempGlobalStmt::Import(_) | TempGlobalStmt::Variable(_) => {}
                item => {
                    let mut item = item.clone();
                    match &mut item {
                        TempGlobalStmt::Func(v) => v.body = None,
                        TempGlobalStmt::Interface(v) => v.body = None,
                        _ => {}
                    }
                    symbols.inner.push((item, *span));
                }
            }
        }
        symbols.units.sort_by(|a, b| a.name.cmp(&b.name));
        symbols.enums.sort_by(|a, b| a.name.cmp(&b.name));
        symbols.functions.sort_by(|a, b| a.name.cmp(&b.name));
        symbols.interfaces.sort_by(|a, b| a.name.cmp(&b.name));
        symbols.generics.sort_by(|a, b| a.name.cmp(&b.name));
        symbols
    }

    /// Hash public signatures plus alias/type dependencies. Ignore private function bodies.
    pub fn public_hash(&self) -> Result<String, String> {
        let mut value = self.clone();
        value.inner.retain(|(item, _)| {
            matches!(
                item,
                TempGlobalStmt::Using(_)
                    | TempGlobalStmt::Unit(_)
                    | TempGlobalStmt::Enum(_)
                    | TempGlobalStmt::Generic(_)
            )
        });
        // Sorting the canonical encoded declarations avoids source-order noise.
        let mut keyed = value
            .inner
            .into_iter()
            .map(|item| Ok((super::semantic_hash(&item)?, item)))
            .collect::<Result<Vec<_>, String>>()?;
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        value.inner = keyed.into_iter().map(|(_, item)| item).collect();
        super::semantic_hash(&value)
    }

    pub fn declarations(&self) -> TempModule {
        let span = (0..0).into();
        let mut module = self.inner.clone();
        module.extend(
            self.units
                .iter()
                .cloned()
                .map(|v| (TempGlobalStmt::Unit(v), span)),
        );
        module.extend(
            self.enums
                .iter()
                .cloned()
                .map(|v| (TempGlobalStmt::Enum(v), span)),
        );
        module.extend(
            self.functions
                .iter()
                .cloned()
                .map(|symbol| (TempGlobalStmt::Func(TempFunc { symbol, body: None }), span)),
        );
        module.extend(self.interfaces.iter().cloned().map(|symbol| {
            (
                TempGlobalStmt::Interface(TempInterface { symbol, body: None }),
                span,
            )
        }));
        module.extend(
            self.generics
                .iter()
                .cloned()
                .map(|v| (TempGlobalStmt::Generic(v), span)),
        );
        module
    }
}

pub fn has_templates(module: &TempModule) -> bool {
    module.iter().any(|(item, _)| match item {
        TempGlobalStmt::Unit(v) => !v.generics.is_empty(),
        TempGlobalStmt::Func(v) => !v.symbol.generics.is_empty(),
        TempGlobalStmt::Interface(v) => !v.symbol.generics.is_empty(),
        TempGlobalStmt::Generic(_) => true,
        _ => false,
    })
}
fn exported(visibility: TempVisibility) -> bool {
    matches!(visibility, TempVisibility::Export | TempVisibility::Api)
}
