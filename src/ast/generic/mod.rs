mod instance;

use crate::ast::TypeIndex;

pub struct GenericInterface {
    pub name: String,
    pub ret_type: TypeIndex,
    pub params: Vec<(TypeIndex, String)>,
}

pub enum GenericRequire {
    Integer,
    Float,
    Signed,
    MinBits(usize),
    MaxBits(usize),
}

pub struct GenericDef {
    pub name: String,
    pub interfaces: Vec<GenericInterface>,
    pub requires: Vec<GenericRequire>,
}
