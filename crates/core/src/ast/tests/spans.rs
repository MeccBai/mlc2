use super::*;
use crate::ast::{
    expression::{Access, CompAtom, Expression, InitialList, UnaryExpr},
    statement::{Statement, Variable},
};

fn variable<'a>(ast: &'a AnalyzedAst, name: &str) -> &'a Variable {
    ast.body
        .iter()
        .flat_map(|function| match function {
            Function::Func(function) => &function.body,
            Function::Interface(function) => &function.body,
        })
        .find_map(|statement| match statement {
            Statement::VariableDecl(variable) if variable.name == name => Some(&**variable),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing variable {name}"))
}

fn fragments<'a>(expression: &Expression, source: &'a str, result: &mut Vec<&'a str>) {
    let span = expression.span();
    assert!(
        span.start < span.end,
        "source expression lost its span: {expression:?}"
    );
    result.push(
        source
            .get(span.into_range())
            .expect("valid UTF-8 source range"),
    );
    match expression {
        Expression::CompositeE(composite, _) => {
            for atom in &composite.members {
                fragments(&atom.clone().to_expression(), source, result);
            }
        }
        Expression::FuncCallE(call, _) => {
            if let Some(callee) = &call.callee {
                fragments(callee, source, result);
            }
            for argument in &call.args {
                fragments(argument, source, result);
            }
        }
        Expression::UnaryExprE(UnaryExpr::Operator { value, .. }, _) => {
            fragments(&value.clone().to_expression(), source, result)
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, .. }), _) => {
            fragments(&base.clone().to_expression(), source, result)
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, index }), _) => {
            fragments(base, source, result);
            fragments(index, source, result);
        }
        Expression::InitListE(
            InitialList::Array { values, .. } | InitialList::List { values, .. },
            _,
        ) => {
            for value in values {
                fragments(value, source, result);
            }
        }
        _ => {}
    }
}

#[test]
fn source_expressions_keep_nested_and_per_reference_spans() {
    let source = "unit Point{pub x:i32;}; func identity(x:i32)->i32{return x;} func main(){var a=1;var first=a;var second=a;var arithmetic=identity(a)+a*(a-1);var items:[i32:2]=[a,3];var indexed=items[0];var negated=-a;var point=Point{a};var field=point.x;}";
    let ast = valid(source);
    let first = variable(&ast, "first").init_val.span();
    let second = variable(&ast, "second").init_val.span();
    assert_eq!(&source[first.into_range()], "a");
    assert_eq!(&source[second.into_range()], "a");
    assert_ne!(first, second);
    assert_eq!(
        variable(&ast, "first").init_val,
        variable(&ast, "second").init_val
    );
    assert_eq!(
        &source[variable(&ast, "a").declaration_span.into_range()],
        "a"
    );
    for (name, expected) in [
        ("arithmetic", "identity(a)+a*(a-1)"),
        ("items", "[a,3]"),
        ("indexed", "items[0]"),
        ("negated", "-a"),
        ("point", "Point{a}"),
        ("field", "point.x"),
    ] {
        let expression = &variable(&ast, name).init_val;
        assert_eq!(&source[expression.span().into_range()], expected, "{name}");
        let mut ranges = Vec::new();
        fragments(expression, source, &mut ranges);
        if name == "arithmetic" {
            assert!(ranges.contains(&"identity(a)"));
            assert!(ranges.contains(&"(a-1)"));
        }
        if !matches!(&**expression, Expression::InitListE(..)) {
            let atom = CompAtom::from_expr((**expression).clone());
            assert_eq!(atom.span(), expression.span());
            assert_eq!(atom.to_expression().span(), expression.span());
        }
    }
}

#[test]
fn folding_and_numeric_normalization_keep_use_site_ranges() {
    let source = "func main(){const original=7;var copied=original;var sum=1+2*3;var wide:i64=4294967296;var numbers:[i64:2]=[4294967296,7];}";
    let mut ast = valid(source);
    for (name, expected) in [
        ("copied", "original"),
        ("sum", "1+2*3"),
        ("wide", "4294967296"),
        ("numbers", "[4294967296,7]"),
    ] {
        let variable = variable(&ast, name);
        let expression = (*variable.init_val).clone();
        let span = expression.span();
        let target = variable.var_type;
        assert_eq!(&source[span.into_range()], expected);
        let folded = expression.const_fold(&mut ast.config, &ast.symbols);
        assert_eq!(folded.span(), span);
        let normalized = folded.normalize_initializer(target, &ast.symbols);
        assert_eq!(normalized.span(), span);
        let mut ranges = Vec::new();
        fragments(&normalized, source, &mut ranges);
    }
}

#[test]
fn functions_and_generic_instances_use_definition_ranges() {
    let generic = "func<T> identity(x:T)->T{return x;}";
    let method = "pub Point::func get(self)->i32{return self->x;}";
    let source = format!(
        "unit Point{{pub x:i32;}}; {generic} {method} func main(){{var a=identity(1);var b=identity(1.5);var p=Point{{1}};var x=p.get();}}"
    );
    let ast = valid(&source);
    let instances: Vec<_> = ast
        .body
        .iter()
        .filter_map(|function| match function {
            Function::Func(function)
                if ast
                    .symbols
                    .functions
                    .get(function.symbol)
                    .name
                    .contains("identity") =>
            {
                Some(function)
            }
            _ => None,
        })
        .collect();
    assert_eq!(instances.len(), 2);
    for instance in instances {
        assert_eq!(&source[instance.span.into_range()], generic);
        assert_eq!(
            ast.symbols.functions.get(instance.symbol).span,
            instance.span
        );
    }
    let template = ast
        .symbols
        .generics
        .functions
        .value_by_ident("identity".into())
        .unwrap();
    assert_eq!(&source[template.span.into_range()], generic);
    for function in &ast.body {
        assert!(function.span().start < function.span().end);
        if let Function::Interface(function) = function {
            assert_eq!(&source[function.span.into_range()], method);
            assert_eq!(
                ast.symbols.interfaces.get(function.symbol).span,
                function.span
            );
        }
    }
}

#[test]
fn pipeline_calls_retain_each_lowered_expression_range() {
    let source = "func id(x:i32)->i32{return x;}func main(){var result=1 |> id |> id;}";
    let ast = valid(source);
    let expression = &variable(&ast, "result").init_val;
    assert_eq!(&source[expression.span().into_range()], "1 |> id |> id");
    let Expression::FuncCallE(outer, _) = &**expression else {
        panic!()
    };
    assert_eq!(&source[outer.args[0].span().into_range()], "1 |> id");
    let Expression::FuncCallE(inner, _) = &outer.args[0] else {
        panic!()
    };
    assert_eq!(&source[inner.args[0].span().into_range()], "1");
}

#[test]
fn unicode_literals_and_following_expressions_use_byte_ranges() {
    let source = "func main(){var text=\"中文\";var count=str::len(text);}";
    let ast = valid(source);
    assert_eq!(
        &source[variable(&ast, "text").init_val.span().into_range()],
        "\"中文\""
    );
    assert_eq!(
        &source[variable(&ast, "count").init_val.span().into_range()],
        "str::len(text)"
    );
}

#[test]
fn generic_interfaces_share_the_template_range() {
    let definition = "api Base::func<T> id(x:T)->T{return x;}";
    let source =
        format!("unit Base{{}};{definition}func main(){{var a=Base::id(1);var b=Base::id(1.5);}}");
    let ast = valid(&source);
    let instances: Vec<_> = ast
        .body
        .iter()
        .filter_map(|function| match function {
            Function::Interface(function) => Some(function),
            _ => None,
        })
        .collect();
    assert_eq!(instances.len(), 2);
    for instance in instances {
        assert_eq!(&source[instance.span.into_range()], definition);
        assert_eq!(
            ast.symbols.interfaces.get(instance.symbol).span,
            instance.span
        );
    }
}
