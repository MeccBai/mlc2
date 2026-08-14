use super::CompileType;
use std::sync::Arc;
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ListType {
    element_type: Arc<CompileType>,
    length: usize,
}
