//! Value-aware scalar initialization. Runtime conversions must preserve the full domain.
use super::Expression;
use crate::ast::{TypeIndex, config::Config, symbols::Resolution};

impl Expression {
    pub(crate) fn numeric_initialization(
        &self,
        target: TypeIndex,
        config: &mut Config,
        symbols: &mut dyn Resolution,
    ) -> Option<bool> {
        crate::TypeInfer::new(config, symbols).numeric_compatible(self, target)
    }

    pub(crate) fn numeric_constant_loss(
        &self,
        target: TypeIndex,
        symbols: &dyn Resolution,
    ) -> bool {
        crate::TypeInfer::numeric_loss(self, target, symbols)
    }

    /// Give accepted constants their destination type before lowering. This avoids
    /// truncating a large literal through its default i32 storage first.
    pub fn normalize_initializer(
        self,
        target: TypeIndex,
        symbols: &(impl crate::ast::types::TypeLookup + ?Sized),
    ) -> Self {
        crate::TypeInfer::normalize_value(self, target, symbols)
    }
}
