use super::Statement;
use crate::{ast::config::Config, diagnostic::warning::Warning};

impl Statement {
    /// Run after the complete body, so references in nested scopes are counted.
    pub(crate) fn warn_unused(body: &[Self], config: &mut Config) {
        if config.is_poisoned() {
            return;
        }
        for statement in body {
            match statement {
                Self::VariableDecl(variable) if variable.read_count.get() == 0 => {
                    config.submit_warning(
                        Warning::UnusedVariable {
                            name: variable.name.clone(),
                        },
                        variable.declaration_span,
                    );
                }
                Self::IfBlock(branch) => {
                    Self::warn_unused(&branch.then_branch, config);
                    if let Some(body) = &branch.else_branch {
                        Self::warn_unused(body, config);
                    }
                }
                Self::WhileBlock(block) => Self::warn_unused(&block.stmts, config),
                // The counted-loop binding is used by its generated condition/update.
                Self::ForBlock(block) => Self::warn_unused(&block.stmts, config),
                Self::MatchBlock(block) => {
                    for (_, body) in &block.branches {
                        Self::warn_unused(body, config);
                    }
                }
                Self::AnonymousBlock(block) => Self::warn_unused(&block.statements, config),
                _ => {}
            }
        }
    }
}
