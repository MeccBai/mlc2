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
            ty: ty.format(symbols),
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
            ty.format(symbols)
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
                    (&f.params, Some((i.file_id(), f.span)), 0)
                }
                EnumBool::True(i) => {
                    let f = symbols.get_interface(i, false);
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
