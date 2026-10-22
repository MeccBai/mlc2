use crate::{
    Error,
    lexer::Token,
    types::{self, CType, DataModel},
};
use mlc_syntax::parser::out::*;
use std::collections::HashSet;

pub(super) struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    end: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, _model: DataModel, end: usize) -> Self {
        Self {
            tokens,
            cursor: 0,
            end,
        }
    }
    pub fn module(mut self) -> Result<TempModule, Error> {
        let mut module = vec![];
        let mut names = HashSet::new();
        while self.cursor < self.tokens.len() {
            let start = self.span().start;
            let definition = self.peek() == "struct"
                && self
                    .tokens
                    .get(self.cursor + 2)
                    .is_some_and(|t| t.text == "{");
            let (item, name) = if definition {
                let unit = self.unit()?;
                let name = unit.name.clone();
                (TempGlobalStmt::Unit(unit), name)
            } else {
                let function = self.function()?;
                let name = function.symbol.name.clone();
                (TempGlobalStmt::Func(function), name)
            };
            if !names.insert(name.clone()) {
                return Err(self.error(format!("Duplicate C symbol: {name}")));
            }
            let end = self.tokens[self.cursor - 1].span.end;
            module.push((item, (start..end).into()));
        }
        Ok(module)
    }

    fn unit(&mut self) -> Result<TempUnit, Error> {
        self.expect("struct")?;
        let (name, name_span) = self.ident()?;
        self.expect("{")?;
        let mut members = vec![];
        let mut names = HashSet::new();
        while self.peek() != "}" {
            let ty = self.ctype()?;
            let (name, name_span) = self.ident()?;
            if !names.insert(name.clone()) {
                return Err(self.error(format!("Duplicate member: {name}")));
            }
            if ty.constant {
                return Err(self.error("const by-value members are not supported"));
            }
            let ty = ty
                .ty
                .ok_or_else(|| self.error("A struct member cannot have type void"))?;
            let ty = self.array(ty)?;
            self.expect(";")?;
            members.push(TempUnitMember {
                name,
                name_span,
                ty,
                public: true,
            });
        }
        self.expect("}")?;
        self.expect(";")?;
        if members.is_empty() {
            return Err(self.error("Empty C structs are not supported"));
        }
        Ok(TempUnit {
            is_union: false,
            visibility: TempVisibility::Export,
            name,
            name_span,
            generics: vec![],
            members,
            attributes: vec!["c_abi".into()],
        })
    }

    fn function(&mut self) -> Result<TempFunc, Error> {
        let returns = self.ctype()?.ty;
        let (name, name_span) = self.ident()?;
        if self.peek() != "(" {
            return Err(self
                .error("Only complete struct definitions and function prototypes are supported"));
        }
        self.expect("(")?;
        let mut params = vec![];
        let mut names = HashSet::new();
        if self.peek() == ")" {
            return Err(self.error("Use (void) for an empty C parameter list"));
        }
        if self.peek() == "void"
            && self
                .tokens
                .get(self.cursor + 1)
                .is_some_and(|t| t.text == ")")
        {
            self.cursor += 1;
        } else {
            loop {
                if self.take("...") {
                    if params.is_empty() {
                        return Err(
                            self.error("A variadic prototype needs at least one fixed parameter")
                        );
                    }
                    let name_span = self.tokens[self.cursor - 1].span.clone().into();
                    params.push(TempParam {
                        name: "...".into(),
                        name_span,
                        ty: None,
                    });
                    break;
                }
                let start = self.span();
                let ty = self
                    .ctype()?
                    .ty
                    .ok_or_else(|| self.error("void is only valid as the entire parameter list"))?;
                let (name, name_span) = if matches!(self.peek(), "," | ")") {
                    (format!("__ffi_arg{}", params.len()), start.into())
                } else {
                    self.ident()?
                };
                if !names.insert(name.clone()) {
                    return Err(self.error(format!("Duplicate parameter: {name}")));
                }
                params.push(TempParam {
                    name,
                    name_span,
                    ty: Some(ty),
                });
                if !self.take(",") {
                    break;
                }
            }
        }
        self.expect(")")?;
        self.expect(";")?;
        Ok(TempFunc {
            body: None,
            symbol: TempFuncSymbol {
                visibility: TempVisibility::Export,
                name,
                name_span,
                generics: vec![],
                params,
                return_type: returns,
                attributes: vec!["c_abi".into()],
            },
        })
    }

    fn ctype(&mut self) -> Result<CType, Error> {
        let start = self.span().start;
        let mut constant = self.take("const");
        let ty = if self.take("struct") {
            let (name, span) = self.ident()?;
            Some(types::path(&name, span))
        } else {
            let ty = types::primitive(self.peek()).ok_or_else(|| self.error("Use explicit fixed-width types such as int32_t or uint8_t; int, char, long, size_t, typedefs and attributes are not supported"))?;
            self.cursor += 1;
            ty.map(|name| types::path(name, (start..self.previous_end()).into()))
        };
        if self.take("const") {
            if constant {
                return Err(self.error("Duplicate const qualifier"));
            }
            constant = true;
        }
        let mut result = CType { ty, constant };
        while self.take("*") {
            let end = self.previous_end();
            let inner = result
                .ty
                .unwrap_or_else(|| types::path("i8", (start..end).into()));
            result.ty = Some((
                TempType::Reference {
                    inner: Box::new(inner),
                    mutable: !result.constant,
                },
                (start..end).into(),
            ));
            result.constant = false;
        }
        Ok(result)
    }

    fn array(&mut self, mut ty: Spanned<TempType>) -> Result<Spanned<TempType>, Error> {
        let mut lengths = vec![];
        while self.take("[") {
            let length = self
                .peek()
                .parse::<usize>()
                .ok()
                .filter(|length| *length > 0)
                .ok_or_else(|| {
                    self.error("Array members require a positive integer literal length")
                })?;
            self.cursor += 1;
            self.expect("]")?;
            lengths.push(length);
        }
        for length in lengths.into_iter().rev() {
            let span = (ty.1.start..self.previous_end()).into();
            ty = (
                TempType::Array {
                    element: Box::new(ty),
                    length,
                },
                span,
            );
        }
        Ok(ty)
    }

    fn ident(&mut self) -> Result<Spanned<String>, Error> {
        let text = self.peek();
        if text.is_empty()
            || !text.as_bytes()[0].is_ascii_alphabetic() && !text.starts_with('_')
            || types::primitive(text).is_some()
            || matches!(
                text,
                "const"
                    | "volatile"
                    | "restrict"
                    | "struct"
                    | "enum"
                    | "union"
                    | "typedef"
                    | "static"
                    | "inline"
                    | "extern"
                    | "void"
                    | "int"
                    | "char"
                    | "float"
                    | "double"
                    | "long"
                    | "short"
                    | "signed"
                    | "unsigned"
                    | "_Bool"
                    | "_Atomic"
                    | "_Complex"
                    | "_Imaginary"
                    | "_Noreturn"
                    | "__cdecl"
                    | "__stdcall"
                    | "__attribute__"
                    | "__declspec"
            )
        {
            return Err(self.error(
                "Expected an identifier; attributes and storage specifiers are not supported",
            ));
        }
        let result = (text.to_owned(), self.span().into());
        self.cursor += 1;
        Ok(result)
    }
    fn peek(&self) -> &str {
        self.tokens
            .get(self.cursor)
            .map_or("", |token| token.text.as_str())
    }
    fn span(&self) -> std::ops::Range<usize> {
        self.tokens
            .get(self.cursor)
            .map_or(self.end..self.end, |token| token.span.clone())
    }
    fn previous_end(&self) -> usize {
        self.cursor
            .checked_sub(1)
            .map_or(0, |index| self.tokens[index].span.end)
    }
    fn take(&mut self, text: &str) -> bool {
        if self.peek() == text {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, text: &str) -> Result<(), Error> {
        if self.take(text) {
            Ok(())
        } else {
            Err(self.error(format!("Expected '{text}', found '{}'", self.peek())))
        }
    }
    fn error(&self, message: impl Into<String>) -> Error {
        Error {
            span: self.span(),
            message: message.into(),
        }
    }
}
