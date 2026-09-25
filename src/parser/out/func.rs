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
    pub symbol: TempFuncSymbol,
    pub body: Option<TempScope>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempFuncSymbol {
    pub visibility: TempVisibility,
    pub owner: Option<TempPath>,
    pub name: String,
    pub generics: Vec<TempGenericParam>,
    pub params: Vec<TempParam>,
    pub return_type: Option<Spanned<TempType>>,
    pub attributes: Vec<String>,
}

impl TempFunc {
    pub fn receiver(&self) -> Option<&TempParam> {
        self.symbol.receiver()
    }

    pub fn has_mutable_receiver(&self) -> bool {
        self.receiver().is_some_and(|receiver| receiver.mutable)
    }

    pub fn split(self) -> (TempFuncSymbol, Option<TempScope>) {
        (self.symbol, self.body)
    }
}

impl TempFuncSymbol {
    pub fn receiver(&self) -> Option<&TempParam> {
        self.params.iter().find(|param| param.is_self)
    }

    pub fn has_mutable_receiver(&self) -> bool {
        self.receiver().is_some_and(|receiver| receiver.mutable)
    }
}
