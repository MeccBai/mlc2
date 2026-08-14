use super::CompileType;
use std::sync::Arc;
#[derive(Debug,Clone,Hash,PartialEq,Eq)]
pub struct RefType {
    base: Arc<CompileType>,
    level: usize,
}
