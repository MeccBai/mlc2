use super::{Span, Spanned, TempParam, TempScope, TempType, TempVisibility};
/// A compile-time function declaration, independent of runtime functions/interfaces.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TempMacroFunc {
    pub visibility: TempVisibility,
    pub name: String,
    pub name_span: Span,
    pub params: Vec<TempParam>,
    pub return_type: Option<Spanned<TempType>>,
    pub body: TempScope,
}
