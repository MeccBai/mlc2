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
