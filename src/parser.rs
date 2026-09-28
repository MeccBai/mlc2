mod expr;
mod func;
mod generic;
mod glob;
pub mod out;
mod split;
mod stmt;

use chumsky::{input::Input, prelude::*};

use crate::lexer::{SpannedToken, TokenPack};

use self::out::Span;

pub use glob::module_parser;
pub(crate) use out::{
    Spanned, TempConstraints as GenericRequirement, TempExpr as Expr, TempFunc as FunctionDecl,
    TempGenericParam as GenericParam, TempLiteralKind as LiteralKind,
    TempMatchPattern as MatchPattern, TempParam as Param, TempPath as Path, TempScope as Scope,
    TempStmt as Statement, TempType as TypeExpr, TempVisibility as Visibility,
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
    use crate::parser::out::{TempEnum, TempExpr, TempMatchPattern, TempPath, TempStmt, TempType};

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
    fn variadic_marker_is_the_last_function_or_interface_parameter() {
        let module = parse_ok(
            "func tt(...) -> int; func mixed(a:i32, ...) -> int; Point::func method(self, a:i32, ...); generic G { func required(self, ...); };",
        );

        let TempGlobalStmt::Func(first) = &module[0].0 else {
            panic!("expected a function");
        };
        assert_eq!(first.symbol.params[0].name, "...");
        assert!(first.symbol.params[0].ty.is_none());

        let TempGlobalStmt::Func(mixed) = &module[1].0 else {
            panic!("expected a function");
        };
        assert_eq!(mixed.symbol.params.len(), 2);
        assert_eq!(mixed.symbol.params[1].name, "...");
        assert!(mixed.symbol.params[1].ty.is_none());

        let TempGlobalStmt::Interface(method) = &module[2].0 else {
            panic!("expected an interface");
        };
        assert!(method.symbol.has_self);
        assert_eq!(method.symbol.params[1].name, "...");
        assert!(method.symbol.params[1].ty.is_none());

        let TempGlobalStmt::Generic(generic) = &module[3].0 else {
            panic!("expected a generic requirement");
        };
        let out::TempConstraints::Interface(required) = &generic.requirements[0].0 else {
            panic!("expected an interface requirement");
        };
        assert_eq!(required.params[0].name, "...");
        assert!(required.params[0].ty.is_none());
    }

    #[test]
    fn rejects_parameters_after_variadic_marker() {
        for source in [
            "func tt(..., a:i32);",
            "func tt(a:i32, ..., b:i32);",
            "Point::func tt(..., a:i32);",
            "generic G { func tt(..., a:i32); };",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "{source} should be rejected");
        }
    }

    #[test]
    fn only_interfaces_accept_pub_and_api_visibility() {
        for declaration in [
            "import std::io;",
            "unit Point {};",
            "using PointAlias = Point;",
            "generic Number {};",
            "enum Color { Red };",
            "func run();",
        ] {
            for visibility in ["pub", "api"] {
                let source = format!("{visibility} {declaration}");
                let lexed = tokenize(&source).unwrap();
                let (_, errors) = parse(&lexed.tokens, source.len());
                assert!(!errors.is_empty(), "{source} should be rejected");
                assert_eq!(errors[0].span().into_range(), 0..visibility.len());
            }
        }

        parse_ok("pub Point::func inspect(self);");
        parse_ok("api Point::func inspect(self);");
        parse_ok("export func run();");
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
    fn variable_declarations_require_initializers() {
        for source in [
            "func main() { var x:i32; }",
            "func main() { const x:i32; }",
            "global var x:i32;",
            "global const x:i32;",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "{source} should be rejected");
        }

        parse_ok("func main() { var x:i32 = 1; const y = 2; }");
        parse_ok("global var x:i32 = 1; global const y = 2;");
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
    fn temp_expressions_flatten_type_preserving_operators_across_precedence() {
        use crate::ast::expression::operators::Operator;

        let module = parse_ok("func main() { var x = a + b - c * d; var y = object.field; }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;
        assert!(matches!(
            &statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Add, Operator::Subtract, Operator::Multiply]
                && operands.len() == 4
        ));
        assert!(matches!(
            &statements[1].0,
            TempStmt::Variable {
                value: Some((
                    TempExpr::Member {
                        indirect: false,
                        ..
                    },
                    _
                )),
                ..
            }
        ));
    }

    #[test]
    fn binary_flattening_preserves_parentheses_and_member_access_kind() {
        use crate::ast::expression::operators::Operator;

        let module = parse_ok("func main() { var x = a - (b - c); var y = ptr->field + 1; }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;
        assert!(matches!(
            &statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Subtract]
                && matches!(&operands[1].0, TempExpr::Group(inner)
                    if matches!(&inner.0, TempExpr::Binary { operators, .. }
                        if operators == &[Operator::Subtract]))
        ));
        assert!(matches!(
            &statements[1].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, .. }, _)),
                ..
            } if matches!(&operands[0].0, TempExpr::Member { indirect: true, .. })
        ));
    }

    #[test]
    fn flattened_binary_keeps_call_init_and_array_operands() {
        let module = parse_ok("func main() { var x = f(1) + Point{2} + [3]; }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let TempStmt::Variable {
            value: Some((TempExpr::Binary { operands, .. }, _)),
            ..
        } = &function.body.as_ref().unwrap().statements[0].0
        else {
            panic!("expected a flattened binary expression");
        };
        assert!(matches!(&operands[0].0, TempExpr::Call { .. }));
        assert!(matches!(&operands[1].0, TempExpr::Init { .. }));
        assert!(matches!(&operands[2].0, TempExpr::Array(_)));
    }

    #[test]
    fn initializer_target_is_a_type_not_an_expression() {
        let source = "func main() { var a = {1, 2}; var b = P{1, 2}; var c = Box<i32>{3}; }";
        let module = parse_ok(source);
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;

        assert!(matches!(
            &statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Init { target: None, .. }, _)),
                ..
            }
        ));
        assert!(matches!(
            &statements[1].0,
            TempStmt::Variable {
                value: Some((TempExpr::Init { target: Some((TempType::Path(path), span)), .. }, _)),
                ..
            } if path.segments == ["P"] && &source[span.into_range()] == "P"
        ));
        assert!(matches!(
            &statements[2].0,
            TempStmt::Variable {
                value: Some((TempExpr::Init {
                    target: Some((TempType::Generic { base, args }, span)),
                    ..
                }, _)),
                ..
            } if base.segments == ["Box"] && args.len() == 1
                && &source[span.into_range()] == "Box<i32>"
        ));
    }

    #[test]
    fn initializer_rejects_an_expression_as_target() {
        for source in [
            "func main() { var x = make_point(){1, 2}; }",
            "func main() { var x = (P){1, 2}; }",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "{source} should be rejected");
        }
    }

    #[test]
    fn comparisons_form_type_changing_binary_boundaries() {
        use crate::ast::expression::operators::Operator;

        let module = parse_ok("func main() { var x = a + b * c < d + e; var y = a & b == c; }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;
        assert!(matches!(
            &statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Less]
                && matches!(&operands[0].0, TempExpr::Binary { operators, .. }
                    if operators == &[Operator::Add, Operator::Multiply])
                && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                    if operators == &[Operator::Add])
        ));
        assert!(matches!(
            &statements[1].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::BitAnd]
                && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                    if operators == &[Operator::Equal])
        ));
    }

    #[test]
    fn logical_operators_keep_boolean_boundaries() {
        use crate::ast::expression::operators::Operator;

        let module = parse_ok("func main() { var x = a < b && c < d; }");
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        assert!(matches!(
            &function.body.as_ref().unwrap().statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::LogicalAnd]
                && operands.iter().all(|(operand, _)| matches!(operand,
                    TempExpr::Binary { operators, .. } if operators == &[Operator::Less]))
        ));
    }

    #[test]
    fn indexing_is_a_type_changing_binary_subexpression() {
        use crate::ast::expression::operators::Operator;

        let module = parse_ok(
            "func main() { var x = values[i + 1] * 2; var y = matrix[row][column]; var z = [7, 8][0]; }",
        );
        let TempGlobalStmt::Func(function) = &module[0].0 else {
            panic!("expected a function");
        };
        let statements = &function.body.as_ref().unwrap().statements;
        assert!(matches!(
            &statements[0].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Multiply]
                && matches!(&operands[0].0, TempExpr::Binary { operands, operators }
                    if operators == &[Operator::Index]
                        && matches!(&operands[1].0, TempExpr::Binary { operators, .. }
                            if operators == &[Operator::Add]))
        ));
        assert!(matches!(
            &statements[1].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Index]
                && matches!(&operands[0].0, TempExpr::Binary { operators, .. }
                    if operators == &[Operator::Index])
        ));
        assert!(matches!(
            &statements[2].0,
            TempStmt::Variable {
                value: Some((TempExpr::Binary { operands, operators }, _)),
                ..
            } if operators == &[Operator::Index]
                && matches!(&operands[0].0, TempExpr::Array(values) if values.len() == 2)
        ));
    }

    #[test]
    fn parses_references_arrays_and_half_open_for() {
        let module = parse_ok(
            "func main() { var a:i32 = 0; var c:$i32 = @a; $c = 10; \
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
                op: crate::ast::expression::operators::Operator::AddressOf,
                ..
            }
        ));
        assert!(matches!(
            &statements[2].0,
            TempStmt::Assignment {
                target: (
                    TempExpr::Unary {
                        op: crate::ast::expression::operators::Operator::Dereference,
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
                .0
                .segments,
            ["number"]
        );

        let TempGlobalStmt::Interface(method) = &module[3].0 else {
            panic!("expected a method");
        };
        assert_eq!(method.symbol.owner.as_ref().unwrap().0.segments, ["Point"]);
        assert!(method.symbol.has_self);
        assert!(method.symbol.mutable);
        assert_eq!(method.symbol.params.len(), 1);
        assert_eq!(method.symbol.params[0].name, "value");

        assert!(matches!(
            &module[4].0,
            TempGlobalStmt::Enum(TempEnum { name, variants, .. })
                if name == "State"
                    && variants.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>()
                        == ["Waiting", "Running"]
        ));
    }

    #[test]
    fn generic_interface_requirement_has_only_a_symbol() {
        let module = parse_ok(
            "generic Addable { pub func add(mut self, rhs:i32) -> i32; Point::func make() -> Point; };",
        );
        let TempGlobalStmt::Generic(generic) = &module[0].0 else {
            panic!("expected a generic declaration");
        };
        let out::TempConstraints::Interface(symbol) = &generic.requirements[0].0 else {
            panic!("expected an interface requirement");
        };
        assert!(symbol.owner.is_none());
        assert!(symbol.has_self);
        assert!(symbol.mutable);
        assert_eq!(symbol.visibility, out::TempVisibility::Public);
        assert_eq!(symbol.name, "add");
        assert_eq!(symbol.params[0].name, "rhs");

        let out::TempConstraints::Interface(owned) = &generic.requirements[1].0 else {
            panic!("expected an owned interface requirement");
        };
        assert_eq!(owned.owner.as_ref().unwrap().0.segments, ["Point"]);
        assert!(!owned.has_self);
        assert!(!owned.mutable);
        assert!(owned.params.is_empty());
    }

    #[test]
    fn preserves_generic_requirement_and_constraint_spans() {
        let source = "generic G { std::generic::max_bits<16>; func check(self); }; unit U<T:G> {};";
        let module = parse_ok(source);
        let TempGlobalStmt::Generic(generic) = &module[0].0 else {
            panic!("expected a generic declaration");
        };
        assert_eq!(
            &source[generic.requirements[0].1.into_range()],
            "std::generic::max_bits<16>;"
        );
        assert_eq!(
            &source[generic.requirements[1].1.into_range()],
            "func check(self);"
        );

        let TempGlobalStmt::Unit(unit) = &module[1].0 else {
            panic!("expected a unit declaration");
        };
        let (_, constraint_span) = unit.generics[0].constraint.as_ref().unwrap();
        assert_eq!(&source[constraint_span.into_range()], "G");
        assert!(!generic.dump().contains("Position:"));
        assert_eq!(generic.dump_with_span().matches("Position:").count(), 2);
    }

    #[test]
    fn preserves_declaration_and_nested_name_spans() {
        let source = "unit Point<T> { x:i32; }; func run(value:i32,...) {} \
                      Point::func get(self, index:i32) {} \
                      generic Numeric { func check(self); }; \
                      enum State { Ready }; using Alias = i32; global var count:i32 = 0;";
        let module = parse_ok(source);
        let at = |span: out::Span| &source[span.into_range()];

        let TempGlobalStmt::Unit(unit) = &module[0].0 else {
            panic!("expected unit");
        };
        assert_eq!(at(unit.name_span), "Point");
        assert_eq!(at(unit.generics[0].name_span), "T");
        assert_eq!(at(unit.members[0].name_span), "x");

        let TempGlobalStmt::Func(function) = &module[1].0 else {
            panic!("expected function");
        };
        assert_eq!(at(function.symbol.name_span), "run");
        assert_eq!(at(function.symbol.params[0].name_span), "value");
        assert_eq!(at(function.symbol.params[1].name_span), "...");

        let TempGlobalStmt::Interface(interface) = &module[2].0 else {
            panic!("expected interface");
        };
        assert_eq!(at(interface.symbol.name_span), "get");
        assert_eq!(at(interface.symbol.params[0].name_span), "index");

        let TempGlobalStmt::Generic(generic) = &module[3].0 else {
            panic!("expected generic");
        };
        assert_eq!(at(generic.name_span), "Numeric");
        let out::TempConstraints::Interface(requirement) = &generic.requirements[0].0 else {
            panic!("expected interface requirement");
        };
        assert_eq!(at(requirement.name_span), "check");

        let TempGlobalStmt::Enum(enumeration) = &module[4].0 else {
            panic!("expected enum");
        };
        assert_eq!(at(enumeration.name_span), "State");
        assert_eq!(at(enumeration.variants[0].1), "Ready");

        let TempGlobalStmt::Using(using) = &module[5].0 else {
            panic!("expected using");
        };
        assert_eq!(at(using.name_span), "Alias");
        let TempGlobalStmt::Variable(variable) = &module[6].0 else {
            panic!("expected global variable");
        };
        assert_eq!(at(variable.name_span), "count");
    }

    #[test]
    fn distinguishes_free_functions_and_interfaces_without_receivers() {
        let module = parse_ok("func make() {} Point::func make() {}");
        assert!(matches!(&module[0].0, TempGlobalStmt::Func(function)
            if function.symbol.name == "make" && function.body.is_some()));
        assert!(matches!(&module[1].0, TempGlobalStmt::Interface(interface)
            if interface.symbol.owner.as_ref().unwrap().0.segments == ["Point"]
                && !interface.symbol.has_self && !interface.symbol.mutable && interface.body.is_some()));
    }

    #[test]
    fn interface_owner_keeps_its_own_span() {
        let source = "pkg::Point::func read(self);";
        let module = parse_ok(source);
        let TempGlobalStmt::Interface(interface) = &module[0].0 else {
            panic!("expected an interface");
        };
        let (owner, span) = interface.symbol.owner.as_ref().unwrap();
        assert_eq!(owner.segments, ["pkg", "Point"]);
        assert_eq!(&source[span.into_range()], "pkg::Point");
    }

    #[test]
    fn receiver_flags_live_only_on_interface_symbol() {
        let module = parse_ok(
            "Point::func read(self, value:i32) {} \
             Point::func write(mut self, value:i32) {} \
             Point::func make(value:i32) {}",
        );
        for ((item, _), (has_self, mutable)) in
            module
                .iter()
                .zip([(true, false), (true, true), (false, false)])
        {
            let TempGlobalStmt::Interface(interface) = item else {
                panic!("expected an interface");
            };
            assert_eq!(interface.symbol.has_self, has_self);
            assert_eq!(interface.symbol.mutable, mutable);
            assert_eq!(interface.symbol.params.len(), 1);
            assert_eq!(interface.symbol.params[0].name, "value");
        }
    }

    #[test]
    fn temp_dump_includes_interface_and_generic_requirements() {
        let module = parse_ok("generic G { func run(self); }; Point::func run(self) {}");
        let generic_dump = module[0].0.dump();
        assert!(generic_dump.contains("Interface Requirement: run"));
        assert!(generic_dump.contains("Has self: true"));

        let interface_dump = module[1].0.dump_with_span(module[1].1);
        assert!(interface_dump.contains("Interface: run"));
        assert!(interface_dump.contains("Owner: Some(\"Point\")"));
        assert!(interface_dump.contains("Position:"));
    }

    #[test]
    fn temp_dump_covers_every_global_item() {
        let module = parse_ok(
            "import std::io; unit Point { x:i32; }; func run() {} \
             Point::func get(self) {} using Number = i32; \
             generic Numeric { std::generic::is_integer; }; \
             enum State { Ready }; global var count:i32 = 0;",
        );
        let labels = [
            "Import:",
            "Unit:",
            "Function:",
            "Interface:",
            "Using:",
            "Generic:",
            "Enum:",
            "Global Variable:",
        ];
        assert_eq!(module.len(), labels.len());
        for ((item, span), label) in module.iter().zip(labels) {
            let dump = item.dump();
            assert!(dump.starts_with(label), "unexpected dump: {dump}");
            assert!(!dump.contains("Position:"), "unexpected span: {dump}");
            let with_span = item.dump_with_span(*span);
            assert!(with_span.contains("Position:"));
        }
        let unit_dump = module[1].0.dump_with_span(module[1].1);
        assert!(unit_dump.contains("Name position:"));
        assert!(unit_dump.contains("Type position:"));
    }

    #[test]
    fn rejects_self_outside_first_interface_parameter() {
        for source in [
            "func invalid(self) {}",
            "Point::func invalid(value:i32, self) {}",
            "generic G { func invalid(value:i32, mut self); };",
        ] {
            let lexed = tokenize(source).unwrap();
            let (_, errors) = parse(&lexed.tokens, source.len());
            assert!(!errors.is_empty(), "accepted invalid receiver: {source}");
        }
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
