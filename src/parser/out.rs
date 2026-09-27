pub mod expr;
pub mod func;
pub mod stmt;
pub mod types;

use crate::{ast::ImportModule, error::ice::ice};

pub use crate::lexer::Span;

pub use expr::{TempExpr, TempLiteralKind};
pub use func::{TempFunc, TempFuncSymbol, TempInterface, TempInterfaceSymbol, TempParam};
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

impl TempVisibility {
    pub fn normal_export(self) -> bool {
        match self {
            Self::Export => true,
            Self::Private => false,
            _ => ice("Unexpected visibility for unit type."),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempGenericParam {
    pub name: String,
    pub constraint: Option<Spanned<TempPath>>,
}

impl TempGenericParam {
    pub fn dump(&self) -> String {
        match &self.constraint {
            Some((path, _)) => format!("{}: {}", self.name, path.segments.join("::")),
            None => self.name.clone(),
        }
    }

    pub fn dump_with_span(&self) -> String {
        match &self.constraint {
            Some((_, span)) => format!("{} @ {span:?}", self.dump()),
            None => self.dump(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempUnitMember {
    pub name: String,
    pub ty: Spanned<TempType>,
    pub public: bool,
}

impl TempUnitMember {
    pub fn dump(&self) -> String {
        format!(
            "Member: {}\n    Public: {}\n    Type: {}",
            self.name,
            self.public,
            self.ty.0.dump()
        )
    }

    pub fn dump_with_span(&self) -> String {
        format!("{}\n    Position: {:?}", self.dump(), self.ty.1)
    }
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

    pub fn dump(&self) -> String {
        self.dump_impl(false)
    }

    pub fn dump_with_span(&self) -> String {
        self.dump_impl(true)
    }

    fn dump_impl(&self, with_span: bool) -> String {
        let mut output = format!(
            "Unit: {}\n    Visibility: {:?}\n    Attributes: {:?}\n    Generics: [{}]",
            self.name,
            self.visibility,
            self.attributes,
            self.generics
                .iter()
                .map(|generic| if with_span {
                    generic.dump_with_span()
                } else {
                    generic.dump()
                })
                .collect::<Vec<_>>()
                .join(", ")
        );
        for member in &self.members {
            let details = if with_span {
                member.dump_with_span()
            } else {
                member.dump()
            };
            output.push_str(&format!("\n    {}", details.replace('\n', "\n    ")));
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempUsing {
    pub visibility: TempVisibility,
    pub name: String,
    pub target: Spanned<TempType>,
}

impl TempUsing {
    pub fn dump(&self) -> String {
        format!(
            "Using: {}\n    Visibility: {:?}\n    Target: {}",
            self.name,
            self.visibility,
            self.target.0.dump()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempConstraints {
    Type {
        path: TempPath,
        argument: Option<String>,
    },
    Interface(TempInterfaceSymbol),
}

impl TempConstraints {
    pub fn dump(&self) -> String {
        match self {
            Self::Type { path, argument } => format!(
                "Type Requirement: {}\n    Argument: {:?}",
                path.segments.join("::"),
                argument
            ),
            Self::Interface(symbol) => format!("Interface Requirement: {}", symbol.dump()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempGeneric {
    pub visibility: TempVisibility,
    pub name: String,
    pub requirements: Vec<Spanned<TempConstraints>>,
    pub attributes: Vec<String>,
}

impl TempGeneric {
    pub fn dump(&self) -> String {
        self.dump_impl(false)
    }

    pub fn dump_with_span(&self) -> String {
        self.dump_impl(true)
    }

    fn dump_impl(&self, with_span: bool) -> String {
        let mut output = format!(
            "Generic: {}\n    Visibility: {:?}\n    Attributes: {:?}",
            self.name, self.visibility, self.attributes
        );
        for (requirement, span) in &self.requirements {
            output.push_str("\n    ");
            output.push_str(&requirement.dump().replace('\n', "\n    "));
            if with_span {
                output.push_str(&format!("\n        Position: {span:?}"));
            }
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempEnum {
    pub visibility: TempVisibility,
    pub name: String,
    pub variants: Vec<String>,
    pub attributes: Vec<String>,
}

impl TempEnum {
    pub fn dump(&self) -> String {
        format!(
            "Enum: {}\n    Visibility: {:?}\n    Variants: {:?}\n    Attributes: {:?}",
            self.name, self.visibility, self.variants, self.attributes
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TempVar {
    pub name: String,
    pub ty: Option<Spanned<TempType>>,
    pub initializer: Spanned<TempExpr>,
    pub constant: bool,
    pub generics: Vec<TempGenericParam>,
}

impl TempVar {
    pub fn dump(&self) -> String {
        format!(
            "Global Variable: {}\n    Constant: {}\n    Type: {}\n    Initializer: {}",
            self.name,
            self.constant,
            self.ty
                .as_ref()
                .map(|(ty, _)| ty.dump())
                .unwrap_or_else(|| "None".into()),
            self.initializer.0.dump()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempGlobalStmt {
    Unit(TempUnit),
    Func(TempFunc),
    Interface(TempInterface),
    Using(TempUsing),
    Generic(TempGeneric),
    Enum(TempEnum),
    Import(ImportModule),
    Variable(TempVar),
}

impl TempGlobalStmt {
    pub fn dump(&self) -> String {
        match self {
            Self::Unit(unit) => unit.dump(),
            Self::Func(function) => function.dump(),
            Self::Interface(interface) => interface.dump(),
            Self::Using(using) => using.dump(),
            Self::Generic(generic) => generic.dump(),
            Self::Enum(enum_) => enum_.dump(),
            Self::Import(import) => format!(
                "Import: {}\n    Exported: {}",
                import.path().join("::"),
                import.exported()
            ),
            Self::Variable(variable) => variable.dump(),
        }
    }

    pub fn dump_with_span(&self, span: Span) -> String {
        let details = match self {
            Self::Unit(unit) => unit.dump_with_span(),
            Self::Generic(generic) => generic.dump_with_span(),
            _ => self.dump(),
        };
        format!("{details}\n    Position: {span:?}")
    }
}
