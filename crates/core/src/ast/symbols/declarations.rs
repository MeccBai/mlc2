use super::SymbolTable;
use crate::ast::{Config, symbol_name::SymbolName};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Spanned, TempGlobalStmt, TempType};

impl SymbolTable {
    /// Preflight source declarations before any arena can overwrite a symbol.
    pub(crate) fn collect_names<'a>(
        &mut self,
        config: &mut Config,
        module: impl Iterator<Item = &'a Spanned<TempGlobalStmt>> + Clone,
    ) {
        for (statement, _) in module.clone() {
            let (name, span) = match statement {
                TempGlobalStmt::Unit(v) => (config.symbol_name(&v.name), v.name_span),
                TempGlobalStmt::Enum(v) => (config.symbol_name(&v.name), v.name_span),
                TempGlobalStmt::Generic(v) => (config.symbol_name(&v.name), v.name_span),
                TempGlobalStmt::Variable(v) => (config.symbol_name(&v.name), v.name_span),
                TempGlobalStmt::Using(v) => (config.symbol_name(&v.name), v.name_span),
                TempGlobalStmt::Func(v) => (config.symbol_name(&v.symbol.name), v.symbol.name_span),
                TempGlobalStmt::Interface(v) => {
                    let owner = v
                        .symbol
                        .owner
                        .as_ref()
                        .map(|(path, _)| config.symbol_name(&SymbolName::path(&path.segments)));
                    (
                        owner
                            .as_deref()
                            .map(|owner| SymbolName::callable(Some(owner), &v.symbol.name))
                            .unwrap_or_else(|| config.symbol_name(&v.symbol.name)),
                        v.symbol.name_span,
                    )
                }
                TempGlobalStmt::Import(_) => continue,
            };
            if crate::ast::builtins::lookup(name.rsplit("::").next().unwrap_or(&name)).is_some() {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::DuplicateSymbol { name }),
                    span,
                );
                return;
            }
            if !self.names.insert(name.clone()) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::DuplicateSymbol { name }),
                    span,
                );
                return;
            }
            let members: Vec<_> = match statement {
                // Members participate in the same fully qualified namespace.
                TempGlobalStmt::Unit(unit) => unit
                    .members
                    .iter()
                    .map(|v| (&v.name, v.name_span))
                    .collect(),
                TempGlobalStmt::Enum(enumeration) => enumeration
                    .variants
                    .iter()
                    .map(|(v, span)| (v, *span))
                    .collect(),
                _ => Vec::new(),
            };
            self.declaration_spans.insert(name.clone(), span);
            for (member, span) in members {
                let member_name = SymbolName::member(&name, member);
                if !self.names.insert(member_name.clone()) {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::DuplicateSymbol {
                            name: member_name,
                        }),
                        span,
                    );
                    return;
                }
                self.declaration_spans.insert(member_name, span);
            }
            if let TempGlobalStmt::Using(using) = statement {
                let TempType::Path(path) = &using.target.0 else {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::UnsupportedUsingTarget),
                        using.target.1,
                    );
                    return;
                };
                let target = SymbolName::path(&path.segments);
                let target = if path.segments.len() == 1 {
                    config.symbol_name(&target)
                } else {
                    target
                };
                self.usings.insert(name, target);
            }
        }
        for (statement, _) in module {
            let TempGlobalStmt::Using(using) = statement else {
                continue;
            };
            if self.expand_using(&config.symbol_name(&using.name)).is_err() {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::CyclicUsing),
                    using.name_span,
                );
                return;
            }
        }
    }

    pub fn expand_using(&self, name: &str) -> Result<String, ()> {
        let mut current = name;
        let mut visited = std::collections::HashSet::new();
        while let Some(target) = self.usings.get(current) {
            if !visited.insert(current) {
                return Err(());
            }
            current = target;
        }
        Ok(current.to_owned())
    }

    pub(crate) fn using_target(&self, config: &Config, name: &str) -> Option<String> {
        let key = config.symbol_name(name);
        self.usings
            .get(name)
            .or_else(|| self.usings.get(&key))
            .and_then(|target| self.expand_using(target).ok())
            .map(|target| {
                let prefix = SymbolName::member(&config.symbol_prefix(), "");
                target.strip_prefix(&prefix).unwrap_or(&target).to_owned()
            })
    }
}
