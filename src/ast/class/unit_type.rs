use std::sync::Arc;
use super::CompileType;
#[derive(Debug,Clone,Hash,PartialEq,Eq)]
pub struct UnitMember {
    pub name: String,
    pub member_type: Arc<CompileType>,
}

#[derive(Debug,Clone,Hash,PartialEq,Eq)]
pub struct UnitType {
    pub name : String,
    pub members: Vec<UnitMember>,
    pub attributes: Vec<String>,
}