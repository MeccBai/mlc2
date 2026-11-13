//! Optional source index, collected before folding discards name references.
use super::{
    TypeIndex,
    config::{Config, FileId},
    expression::{Access, Expression, UnaryExpr},
    symbols::{EnumBool, Resolution},
    types::{CompileType, TypeLookup},
};
use crate::parser::out::Span;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub symbol: Option<String>,
    pub span: Span,
    pub ty: String,
    pub kind: &'static str,
    pub definition: Option<(FileId, Span)>,
    pub parameters: Vec<(Span, String)>,
}
pub fn type_occurrence(config: &mut Config, span: Span, ty: TypeIndex, symbols: &dyn Resolution) {
    if config.semantic.is_none() || ty.is_empty() {
        return;
    }
    let t = symbols.get_type(ty).unqualified();
    let (kind, name) = match t {
        CompileType::Unit(u) => (
            if u.variant.is_some() { "type" } else { "class" },
            Some(u.name.as_str()),
        ),
        CompileType::Enum(e) => ("enum", Some(e.name.as_str())),
        _ => ("type", None),
    };
    let definition = name
        .and_then(|name| {
            symbols
                .table(ty.file_id())
                .declaration_span(name.split('<').next().unwrap_or(name))
        })
        .map(|s| (ty.file_id(), s));
    config
        .semantic
        .as_ref()
        .unwrap()
        .borrow_mut()
        .push(Occurrence {
            symbol: None,
            span,
            ty: display_type(ty, symbols),
            kind,
            definition,
            parameters: vec![],
        });
}
pub fn expression(config: &mut Config, value: &Expression, symbols: &mut dyn Resolution) {
    if config.semantic.is_none() || config.is_poisoned() {
        return;
    }
    let ty = value.type_inference(config, symbols);
    let mut occurrence = Occurrence {
        symbol: None,
        span: value.span(),
        ty: if ty.is_empty() {
            "void".into()
        } else {
            display_type(ty, symbols)
        },
        kind: "variable",
        definition: None,
        parameters: vec![],
    };
    match value {
        Expression::VarValueE(v, _) => {
            if v.declaration_span.end == 0 {
                occurrence.kind = "parameter";
            }
            occurrence.definition = Some((config.file_id(), v.declaration_span));
        }
        Expression::FuncCallE(call, _) => {
            occurrence.kind = "function";
            let (params, definition, offset) = match call.func {
                EnumBool::False(i) => {
                    let f = symbols.get_function_regular(i);
                    occurrence.symbol = Some(f.name.clone());
                    occurrence.ty = function_signature(
                        &f.name,
                        f.constant,
                        &f.generics,
                        &f.params,
                        f.ret_type,
                        symbols,
                    );
                    (&f.params, Some((i.file_id(), f.span)), 0)
                }
                EnumBool::True(i) => {
                    let f = symbols.get_interface(i, false);
                    occurrence.symbol = Some(f.name.clone());
                    occurrence.ty = function_signature(
                        &f.name,
                        false,
                        &f.generics,
                        &f.params,
                        f.ret_type,
                        symbols,
                    );
                    (
                        &f.params,
                        Some((i.file_id(), f.span)),
                        usize::from(f.receiver_type.is_some()),
                    )
                }
            };
            occurrence.definition = definition;
            occurrence.parameters = call
                .args
                .iter()
                .skip(offset)
                .zip(params)
                .filter(|(_, p)| p.1 != "...")
                .map(|(arg, p)| (arg.span(), p.1.clone()))
                .collect();
        }
        Expression::UnaryExprE(
            UnaryExpr::Access(Access::Member {
                base,
                name,
                indirect,
            }),
            _,
        ) => {
            occurrence.kind = "property";
            let mut owner = base.type_inference(config, symbols);
            if *indirect && !owner.is_empty() {
                owner = owner.deref(symbols).unwrap_or(owner);
            }
            if !owner.is_empty() {
                if let CompileType::Unit(unit) = symbols.get_type(owner).unqualified() {
                    occurrence.symbol = Some(format!("{}::{name}", unit.name));
                }
            }
        }
        Expression::ConstValueE(value, _) => {
            occurrence.kind = "number";
            if !ty.is_empty() {
                if let CompileType::Enum(e) = symbols.get_type(ty).unqualified() {
                    if let Ok(i) = value.value.parse::<usize>() {
                        if let Some(name) = e.variants.get(i) {
                            occurrence.kind = "enumMember";
                            occurrence.symbol = Some(format!("{}::{name}", e.name));
                        }
                    }
                }
            }
        }
        _ => {}
    }
    config
        .semantic
        .as_ref()
        .unwrap()
        .borrow_mut()
        .push(occurrence);
}

/// Source-facing names omit module qualifiers; internal identity remains unchanged.
pub fn short_name(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\0' {
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            let mut word = String::from(c);
            while chars
                .peek()
                .is_some_and(|c| c.is_alphanumeric() || *c == '_')
            {
                word.push(chars.next().unwrap());
            }
            loop {
                let mut look = chars.clone();
                if look.next() != Some(':')
                    || look.next() != Some(':')
                    || !look
                        .peek()
                        .is_some_and(|c| c.is_alphanumeric() || *c == '_')
                {
                    break;
                }
                chars = look;
                word.clear();
                while chars
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || *c == '_')
                {
                    word.push(chars.next().unwrap());
                }
            }
            out.push_str(&word);
        } else {
            out.push(c);
        }
    }
    out
}
pub fn display_type(ty: TypeIndex, symbols: &(impl TypeLookup + ?Sized)) -> String {
    if ty.is_empty() {
        return "void".into();
    }
    short_name(&symbols.get_type(ty).unqualified().format(symbols))
}
pub fn function_signature(
    name: &str,
    constant: bool,
    generics: &[String],
    params: &[(TypeIndex, String)],
    returns: Option<TypeIndex>,
    symbols: &(impl TypeLookup + ?Sized),
) -> String {
    let parameters = params
        .iter()
        .map(|(ty, name)| {
            if name == "..." {
                "...".into()
            } else {
                format!("{name}: {}", display_type(*ty, symbols))
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let generic = if generics.is_empty() {
        String::new()
    } else {
        format!("<{}>", generics.join(", "))
    };
    let returns = returns
        .map(|ty| format!(" -> {}", display_type(ty, symbols)))
        .unwrap_or_default();
    format!(
        "{}func {}{generic}({parameters}){returns}",
        if constant { "const " } else { "" },
        short_name(name)
    )
}
