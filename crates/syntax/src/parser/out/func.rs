use super::{Span, Spanned, TempGenericParam, TempPath, TempScope, TempType, TempVisibility};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempParam {
    pub name: String,
    pub name_span: Span,
    pub ty: Option<Spanned<TempType>>,
}

impl TempParam {
    pub fn dump(&self) -> String {
        match &self.ty {
            Some((ty, _)) => format!("{}: {}", self.name, ty.dump()),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempFunc {
    pub symbol: TempFuncSymbol,
    pub body: Option<TempScope>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempFuncSymbol {
    pub visibility: TempVisibility,
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<TempGenericParam>,
    pub params: Vec<TempParam>,
    pub return_type: Option<Spanned<TempType>>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempInterface {
    pub symbol: TempInterfaceSymbol,
    pub body: Option<TempScope>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempInterfaceSymbol {
    pub visibility: TempVisibility,
    pub owner: Option<Spanned<TempPath>>,
    pub has_self: bool,
    pub mutable: bool,
    pub name: String,
    pub name_span: Span,
    pub generics: Vec<TempGenericParam>,
    pub params: Vec<TempParam>,
    pub return_type: Option<Spanned<TempType>>,
    pub attributes: Vec<String>,
}

impl TempFunc {
    pub fn split(self) -> (TempFuncSymbol, Option<TempScope>) {
        (self.symbol, self.body)
    }

    pub fn dump(&self) -> String {
        format!(
            "Function: {}\n    Body: {}",
            self.symbol.dump(),
            self.body.is_some()
        )
    }
}

impl TempInterface {
    pub fn split(self) -> (TempInterfaceSymbol, Option<TempScope>) {
        (self.symbol, self.body)
    }

    pub fn dump(&self) -> String {
        format!(
            "Interface: {}\n    Body: {}",
            self.symbol.dump(),
            self.body.is_some()
        )
    }
}

impl TempFuncSymbol {
    pub fn dump(&self) -> String {
        format!(
            "{}\n    Visibility: {:?}\n    Generics: [{}]\n    Parameters: [{}]\n    Return Type: {}\n    Attributes: {:?}",
            self.name,
            self.visibility,
            self.generics
                .iter()
                .map(TempGenericParam::dump)
                .collect::<Vec<_>>()
                .join(", "),
            self.params
                .iter()
                .map(TempParam::dump)
                .collect::<Vec<_>>()
                .join(", "),
            self.return_type
                .as_ref()
                .map(|(ty, _)| ty.dump())
                .unwrap_or_else(|| "None".into()),
            self.attributes
        )
    }
}

impl TempInterfaceSymbol {
    pub fn dump(&self) -> String {
        format!(
            "{}\n    Owner: {:?}\n    Has self: {}\n    Mutable: {}\n    Visibility: {:?}\n    Generics: [{}]\n    Parameters: [{}]\n    Return Type: {}\n    Attributes: {:?}",
            self.name,
            self.owner
                .as_ref()
                .map(|(owner, _)| owner.segments.join("::")),
            self.has_self,
            self.mutable,
            self.visibility,
            self.generics
                .iter()
                .map(TempGenericParam::dump)
                .collect::<Vec<_>>()
                .join(", "),
            self.params
                .iter()
                .map(TempParam::dump)
                .collect::<Vec<_>>()
                .join(", "),
            self.return_type
                .as_ref()
                .map(|(ty, _)| ty.dump())
                .unwrap_or_else(|| "None".into()),
            self.attributes
        )
    }
}
