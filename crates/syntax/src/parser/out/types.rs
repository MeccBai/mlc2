use super::Spanned;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TempPath {
    pub segments: Vec<String>,
}

impl TempPath {
    pub fn join(self) -> String {
        self.segments.join("::")
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum TempType {
    Function {
        params: Vec<Spanned<TempType>>,
        returns: Option<Box<Spanned<TempType>>>,
        variadic: bool,
    },
    InferredResource,
    Resource {
        inner: Box<Spanned<TempType>>,
    },
    Array {
        element: Box<Spanned<TempType>>,
        length: usize,
    },
    Path(TempPath),
    Generic {
        base: TempPath,
        args: Vec<Spanned<TempType>>,
    },
    Reference {
        inner: Box<Spanned<TempType>>,
        mutable: bool,
    },
}

impl TempType {
    pub fn dump(&self) -> String {
        match self {
            Self::Function {
                params,
                returns,
                variadic,
            } => {
                let mut params = params.iter().map(|ty| ty.0.dump()).collect::<Vec<_>>();
                if *variadic {
                    params.push("...".into());
                }
                let returns = returns
                    .as_ref()
                    .map(|ty| format!(" -> {}", ty.0.dump()))
                    .unwrap_or_default();
                format!("func({}){returns}", params.join(", "))
            }
            Self::InferredResource => "res".into(),
            Self::Resource { inner } => format!("res {}", inner.0.dump()),
            Self::Array { element, length } => format!("[{}:{length}]", element.0.dump()),
            Self::Path(path) => path.segments.join("::"),
            Self::Generic { base, args } => format!(
                "{}<{}>",
                base.segments.join("::"),
                args.iter()
                    .map(|(arg, _)| arg.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Reference { inner, mutable } => {
                if *mutable {
                    format!("$mut {}", inner.0.dump())
                } else {
                    format!("${}", inner.0.dump())
                }
            }
        }
    }
}
