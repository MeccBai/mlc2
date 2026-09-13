use crate::ast::TypeIndex;
use std::sync::Arc;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ListType {
    element_type: TypeIndex,
    length: usize,
}
