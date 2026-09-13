use super::{Spanned, TempGenericParam, TempPath, TempScope, TempType, TempVisibility};

#[derive(Debug, Clone, PartialEq)]
pub struct TempParam {
    pub name: String,
    pub ty: Option<Spanned<TempType>>,
    pub is_self: bool,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempFunc {
    pub visibility: TempVisibility,
    pub owner: Option<TempPath>,
    pub name: String,
    pub generics: Vec<TempGenericParam>,
    pub params: Vec<TempParam>,
    pub return_type: Option<Spanned<TempType>>,
    pub body: Option<TempScope>,
    pub attributes: Vec<String>,
}
