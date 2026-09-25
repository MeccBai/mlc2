pub mod expr;
pub mod func;
pub mod stmt;
pub mod types;

use crate::{ast::ImportModule, lexer::Span};

pub use expr::{TempBinaryOp, TempExpr, TempLiteralKind, TempMemberAccess, TempUnaryOp};
pub use func::{TempFunc, TempFuncSymbol, TempParam};
pub use stmt::{TempMatchPattern, TempScope, TempStmt};
pub use types::{TempPath, TempType};

pub type Spanned<T> = (T, Span);
pub type TempModule = Vec<Spanned<TempGlobalStmt>>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TempVisibility {
    #[default]
    Private,
    Public,
    Export,
    Api,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempGenericParam {
    pub name: String,
    pub constraint: Option<TempPath>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempUnitMember {
    pub name: String,
    pub ty: Spanned<TempType>,
    pub public: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempUnit {
    pub visibility: TempVisibility,
    pub name: String,
    pub generics: Vec<TempGenericParam>,
    pub members: Vec<TempUnitMember>,
    pub attributes: Vec<String>,
}

impl TempUnit {
    pub fn has_generic(&self) -> bool {
        !self.generics.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempUsing {
    pub visibility: TempVisibility,
    pub name: String,
    pub target: Spanned<TempType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempConstraints {
    Type {
        path: TempPath,
        argument: Option<String>,
    },
    Function(TempFunc),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempGeneric {
    pub visibility: TempVisibility,
    pub name: String,
    pub requirements: Vec<TempConstraints>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempEnum {
    pub visibility: TempVisibility,
    pub name: String,
    pub variants: Vec<String>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempVar {
    pub name: String,
    pub ty: Option<Spanned<TempType>>,
    pub initializer: Option<Spanned<TempExpr>>,
    pub constant: bool,
    pub generics: Vec<TempGenericParam>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempGlobalStmt {
    Unit(TempUnit),
    Func(TempFunc),
    Using(TempUsing),
    Generic(TempGeneric),
    Enum(TempEnum),
    Import(ImportModule),
    Variable(TempVar),
}
