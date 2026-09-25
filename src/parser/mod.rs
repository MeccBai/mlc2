mod expr;
mod func;
mod generic;
mod glob;
pub mod out;
mod split;
mod stmt;

use chumsky::{input::Input, prelude::*};

use crate::lexer::{Span, SpannedToken, TokenPack};

pub use glob::module_parser;
pub(crate) use out::{
    Spanned, TempBinaryOp as BinaryOp, TempConstraints as GenericRequirement, TempExpr as Expr,
    TempFunc as FunctionDecl, TempGenericParam as GenericParam, TempLiteralKind as LiteralKind,
    TempMatchPattern as MatchPattern, TempMemberAccess as MemberAccess, TempParam as Param,
    TempPath as Path, TempScope as Scope, TempStmt as Statement, TempType as TypeExpr,
    TempUnaryOp as UnaryOp, TempVisibility as Visibility,
};
pub use out::{TempGlobalStmt, TempModule};

pub type ParseError<'tokens> = Rich<'tokens, TokenPack, Span>;

pub fn parse(
    tokens: &[SpannedToken],
    source_len: usize,
) -> (Option<TempModule>, Vec<ParseError<'_>>) {
    let end_span: Span = (source_len..source_len).into();
    let input = tokens.map(end_span, |(token, span)| (token, span));
    module_parser().parse(input).into_output_errors()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::out::{
        TempEnum, TempExpr, TempMatchPattern, TempPath, TempStmt, TempType, TempUnaryOp,
    };

    fn parse_ok(source: &str) -> TempModule {
        let lexed = tokenize(source).unwrap();
        let (module, errors) = parse(&lexed.tokens, source.len());
        assert!(errors.is_empty(), "{errors:#?}");
        module.expect("valid source should produce a module")
    }

    #[test]
    fn parses_import() {
        assert_eq!(parse_ok("import std::io;").len(), 1);
    }

    #[test]
    fn parser_keeps_using_only_as_a_temp_statement() {
        let module = parse_ok("using bytes = Buffer<u8>; ");
        let TempGlobalStmt::Using(using) = &module[0].0 else {
            panic!("expected a temporary using declaration");
        };

        assert_eq!(using.name, "bytes");
        assert!(matches!(using.target.0, TempType::Generic { .. }));
    }

    #[test]
    fn parses_global_variables_without_export_visibility() {
        let module = parse_ok("global var counter:i32 = 0; global const limit:i32 = 10;");

        assert!(matches!(
            &module[0].0,
            TempGlobalStmt::Variable(variable)
                if variable.name == "counter" && !variable.constant
        ));
        assert!(matches!(
            &module[1].0,
            TempGlobalStmt::Variable(variable)
                if variable.name == "limit" && variable.constant
        ));
    }

    #[test]
    fn parses_public_and_private_unit_members() {
        let module = parse_ok("unit Point { pub x:i32; y:i32; };");
        let TempGlobalStmt::Unit(unit) = &module[0].0 else {
            panic!("expected a unit");
        };

        assert!(unit.members[0].public);
        assert!(!unit.members[1].public);
    }

    #[test]
    fn rejects_export_as_unit_member_visibility() {
        let source = "unit Point { export x:i32; };";
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }

    #[test]
    fn rejects_exported_global_variables() {
        for source in [
            "export global var counter:i32 = 0;",
            "api global var counter:i32 = 0;",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "accepted invalid source: {source}");
        }
    }

    #[test]
    fn lowers_pipe_to_free_function_call() {
        let source = "func main() { var result = {1, 2} |> std::sum(3); }";
        let module = parse_ok(source);
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let Some(TempStmt::Variable {
            value: Some((TempExpr::Call { callee, args }, _)),
            ..
        }) = function
            .body
            .as_ref()
            .and_then(|body| body.statements.first())
            .map(|statement| &statement.0)
        else {
            panic!("expected the pipeline to lower to a call");
        };

        assert_eq!(args.len(), 3);
        assert!(matches!(
            &callee.0,
            TempExpr::Path(TempPath { segments }) if segments == &["std", "sum"]
        ));
    }

    #[test]
    fn rejects_member_function_pipeline() {
        let source = "func main() { x |> object.f(a); }";
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }

    #[test]
    fn parses_references_arrays_and_half_open_for() {
        let module = parse_ok(
            "func main() { var a:i32; var c:$i32 = @a; $c = 10; \
             var values = [1, 2, 3]; for i in [0, 10] { continue; } }",
        );
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;

        let TempStmt::Variable {
            ty: Some((ty, _)),
            value: Some((value, _)),
            ..
        } = &statements[1].0
        else {
            panic!("expected a referenced variable");
        };
        assert!(matches!(ty, TempType::Reference(_)));
        assert!(matches!(
            value,
            TempExpr::Unary {
                op: TempUnaryOp::AddressOf,
                ..
            }
        ));
        assert!(matches!(
            &statements[2].0,
            TempStmt::Assignment {
                target: (
                    TempExpr::Unary {
                        op: TempUnaryOp::Dereference,
                        ..
                    },
                    _
                ),
                ..
            }
        ));
        assert!(matches!(
            &statements[3].0,
            TempStmt::Variable { value: Some((TempExpr::Array(values), _)), .. }
                if values.len() == 3
        ));
        assert!(matches!(
            &statements[4].0,
            TempStmt::For { binding, .. } if binding == "i"
        ));
    }

    #[test]
    fn parses_explicit_generics_method_mutability_enum_and_c_abi() {
        let module = parse_ok(
            "generic number { std::generic::is_integer; }; \
             [[c_abi]] func<T:number> identity(value:T) -> T { return value; } \
             unit Point { x:i32; }; Point::func set(mut self, value:i32) {} \
             enum State { Waiting, Running };",
        );

        let TempGlobalStmt::Func(function) = &module[1].0 else {
            panic!("expected a free function");
        };
        assert_eq!(function.symbol.attributes, ["c_abi"]);
        assert_eq!(function.symbol.generics[0].name, "T");
        assert_eq!(
            function.symbol.generics[0]
                .constraint
                .as_ref()
                .unwrap()
                .segments,
            ["number"]
        );

        let TempGlobalStmt::Func(method) = &module[3].0 else {
            panic!("expected a method");
        };
        assert_eq!(method.symbol.owner.as_ref().unwrap().segments, ["Point"]);
        assert!(method.symbol.params[0].is_self && method.symbol.params[0].mutable);
        assert!(method.has_mutable_receiver());

        assert!(matches!(
            &module[4].0,
            TempGlobalStmt::Enum(TempEnum { name, variants, .. })
                if name == "State" && variants == &["Waiting", "Running"]
        ));
    }

    #[test]
    fn rejects_explicit_reference_syntax_for_method_receiver() {
        for source in [
            "Point::func set(self:$Point) {}",
            "Point::func set(mut self:$Point) {}",
            "Point::func set(&self) {}",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "accepted invalid receiver: {source}");
        }
    }

    #[test]
    fn parses_default_match_and_only_explicit_anonymous_blocks() {
        let module = parse_ok("func main() { match (value) { _ => {} } anonymous {} }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        assert!(matches!(
            &function.body.as_ref().unwrap().statements[0].0,
            TempStmt::Match { branches, .. }
                if matches!(branches[0].0, TempMatchPattern::Default)
        ));
        assert!(matches!(
            &function.body.as_ref().unwrap().statements[1].0,
            TempStmt::Anonymous(_)
        ));

        let source = "func main() { {} }";
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }

    #[test]
    fn rejects_deprecated_range_operator() {
        let source = "func main() { for i in 0..10 {} }";
        let lexed = tokenize(source).unwrap();
        let (_, errors) = parse(&lexed.tokens, source.len());
        assert!(!errors.is_empty());
    }
}
