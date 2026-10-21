//! Literal-only conversion to the canonical non-owning UTF-8 view.
use crate::ast::{
    TypeIndex,
    config::Config,
    expression::{ConstValue, Expression, InitialList},
    symbols::Resolution,
    types::{CompileType, base_type::DataType},
};
use crate::parser::out::{Span, TempExpr, TempLiteralKind};
impl Expression {
    pub(crate) fn coerce_string_view(
        self,
        target: TypeIndex,
        config: &mut Config,
        symbols: &mut dyn Resolution,
        _span: Span,
    ) -> Self {
        let source_span = self.span();
        if target.is_empty() {
            return self;
        }
        let CompileType::Unit(unit) = symbols.get_type(target).unqualified() else {
            return self;
        };
        if unit.name != "std::base::ConstStringView" {
            return self;
        }
        let fields: Vec<_> = unit.members.iter().map(|m| m.member_type).collect();
        if fields.len() != 2 {
            return self;
        }
        let value = self.const_fold(config, symbols);
        let Self::InitListE(InitialList::String { value: text }, ..) = &value else {
            return value;
        };
        let value = text;
        let length = value.len();
        let mut literal = String::from("\"");
        // Source literals accept this exact set of escapes.
        for ch in value.chars() {
            match ch {
                '\n' => literal.push_str("\\n"),
                '\r' => literal.push_str("\\r"),
                '\t' => literal.push_str("\\t"),
                '\0' => literal.push_str("\\0"),
                '\\' => literal.push_str("\\\\"),
                '"' => literal.push_str("\\\""),
                _ => literal.push(ch),
            }
        }
        literal.push('"');
        let pointer = Self::new_builtin_call(
            config,
            crate::ast::builtins::Builtin::ConstCStr,
            TypeIndex::empty(),
            vec![(
                TempExpr::Literal {
                    kind: TempLiteralKind::String,
                    text: literal,
                },
                source_span,
            )],
            source_span,
            symbols,
            None,
        ).with_span(source_span);
        Self::InitListE(InitialList::List {
            onwer: Some(target),
            values: vec![
                pointer,
                Self::ConstValueE(ConstValue {
                    value: length.to_string(),
                    ty: fields[1],
                }, source_span),
            ],
        }, source_span)
    }
    pub(crate) fn coerce_arguments(
        args: Vec<Self>,
        parameters: &[(TypeIndex, String)],
        config: &mut Config,
        symbols: &mut dyn Resolution,
        span: Span,
    ) -> Vec<Self> {
        args.into_iter()
            .enumerate()
            .map(|(i, arg)| {
                if let Some((ty, _)) = parameters.get(i) {
                    arg.coerce_string_view(*ty, config, symbols, span)
                } else {
                    arg
                }
            })
            .collect()
    }
}
