use super::*;
use ast::{
    config::{Config, FileId},
    expression::Expression,
    statement::Statement,
};
use mlc_core::diagnostic::{error::ErrorHandle, warning::WarningHandle};

fn analyze(source: &str) -> (AbstractSyntaxTree, PackageSymbolTable) {
    let tokens = mlc_syntax::lexer::tokenize(source).unwrap();
    let (module, errors) = mlc_syntax::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let config = Config::new(
        FileId::new(0),
        vec![],
        String::new(),
        "test.m2".into(),
        ErrorHandle::new("test.m2".into()),
        WarningHandle::new("test.m2".into()),
    );
    let mut ast = AbstractSyntaxTree::new(config, module.unwrap());
    let mut package = PackageSymbolTable::new();
    ast.analysis(&mut package);
    (ast, package)
}

fn optimize(source: &str, config: OptimizeConfig) -> (AbstractSyntaxTree, PackageSymbolTable) {
    let (ast, mut package) = analyze(source);
    assert!(!ast.config.is_poisoned(), "{:?}", ast.config.error_handle());
    (Optimizer::new(config).optimize(ast, &mut package), package)
}

#[test]
fn optional_constant_folding_does_not_control_semantic_checks() {
    let source = "func main(){var x=1+2*3;const required=1+2;}";
    let (mut unoptimized, _) = optimize(source, OptimizeConfig::disabled());
    let body = body_mut(&mut unoptimized.body[0]);
    let Statement::VariableDecl(variable) = &body[0] else {
        panic!()
    };
    assert!(matches!(*variable.init_val, Expression::CompositeE(_, ..)));
    let Statement::VariableDecl(variable) = &body[1] else {
        panic!()
    };
    assert!(matches!(*variable.init_val, Expression::ConstValueE(_, ..)));
    let (mut optimized, _) = optimize(source, OptimizeConfig::default());
    let Statement::VariableDecl(variable) = &body_mut(&mut optimized.body[0])[0] else {
        panic!()
    };
    assert!(matches!(&*variable.init_val, Expression::ConstValueE(value, ..) if value.value=="7"));
    for source in [
        "func main(){var x:i8=150+150;}",
        "func main(){var x:i8=300;}",
    ] {
        assert!(analyze(source).0.config.is_poisoned());
    }
}

#[test]
fn disabling_all_passes_preserves_ast_statement_shapes() {
    let (ast, mut package) =
        analyze("func effect(){}func main(){effect();var value=1+2;if(true){effect();}}");
    let before = ast.clone();
    let after = Optimizer::new(OptimizeConfig::disabled()).optimize(ast, &mut package);
    assert_eq!(before, after);
}

#[test]
fn control_flow_pass_is_independent_and_keeps_anonymous_scope() {
    let source = "func main(){if(1<2){var a=1;}else{var b=2;}while(false){}for i in [4,2]{}match(2){1=>{},_=>{}}}";
    let mut config = OptimizeConfig::disabled();
    let (mut ast, _) = optimize(source, config);
    assert!(matches!(
        body_mut(&mut ast.body[0])[0],
        Statement::IfBlock(_)
    ));
    config.dead_code_elimination = true;
    let (mut ast, _) = optimize(source, config);
    assert!(
        body_mut(&mut ast.body[0])
            .iter()
            .all(|statement| matches!(statement, Statement::AnonymousBlock(_)))
    );
}

#[test]
fn unused_variables_preserve_effects_owners_and_address_taken_storage() {
    let mut config = OptimizeConfig::disabled();
    config.unused_variable_elimination = true;
    let (mut ast, _) = optimize(
        "func effect()->i32{return 1;}func main(p:res $mut i32){var unused=1;var effectful=effect();var owned:res=p;var x=1;var r=@mut x;$r=2;}",
        config,
    );
    let body = body_mut(ast.body.last_mut().unwrap());
    assert!(matches!(body[0], Statement::AnonymousBlock(_)));
    assert!(matches!(
        body[1],
        Statement::Expression(Expression::FuncCallE(_, ..))
    ));
    assert!(matches!(&body[2], Statement::VariableDecl(variable) if variable.name=="owned"));
    assert!(matches!(&body[3], Statement::VariableDecl(variable) if variable.name=="x"));
}

#[test]
fn unused_functions_keep_calls_addresses_exports_and_global_initializers() {
    let source = "func dead(){} func used(){} func address(){} export func exported_fn(){} func initialize()->i32{return 1;}global var value=initialize();func main(){used();var pointer=std::function(address);}";
    let (original, _) = optimize(source, OptimizeConfig::disabled());
    let mut config = OptimizeConfig::disabled();
    config.unused_function_elimination = true;
    let (ast, package) = optimize(source, config);
    assert_eq!(original.body.len(), ast.body.len() + 1);
    let names = ast
        .body
        .iter()
        .map(|function| match function {
            Function::Func(body) => package.get_function(body.symbol, false).name.as_str(),
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    assert!(!names.iter().any(|name| name.ends_with("dead")));
    for expected in ["used", "address", "exported_fn", "initialize", "main"] {
        assert!(
            names.iter().any(|name| name.ends_with(expected)),
            "{names:?}"
        );
    }
}

#[test]
fn generic_instances_and_supplement_share_the_optimized_body() {
    let (ast, package) = optimize(
        "func<T> id(x:T)->T{if(true){return x;}else{return x;}}func main(){var x=id<i32>(1);}",
        OptimizeConfig::default(),
    );
    let instance = package
        .file(ast.config.file_id())
        .unwrap()
        .function_instances
        .values()
        .next()
        .unwrap();
    assert!(matches!(instance.body[0], Statement::AnonymousBlock(_)));
}

#[test]
fn folding_globals_rebinds_dependent_initializers_and_generic_bodies() {
    let (mut ast, package) = optimize(
        "global var first=1+2;global var second=first;func<T> read(x:T)->i32{return second+first;}func main()->i32{return read<i32>(0);}",
        OptimizeConfig::default(),
    );
    let file = package.file(ast.config.file_id()).unwrap();
    let first = &file.globals["first"].1;
    let second = &file.globals["second"].1;
    assert!(matches!(&*first.init_val, Expression::ConstValueE(value, ..) if value.value=="3"));
    assert!(
        matches!(&*second.init_val, Expression::VarValueE(variable, ..) if std::rc::Rc::ptr_eq(variable, first))
    );
    let mut references = 0;
    for function in &mut ast.body {
        walk::expressions(body_mut(function), &mut |value| {
            if let Expression::VarValueE(variable, ..) = value {
                if variable.name.ends_with("first") || variable.name.ends_with("second") {
                    assert!(package.global_owner(variable).is_some());
                    references += 1;
                }
            }
        });
    }
    assert!(references >= 2);
}

#[test]
fn source_ranges_survive_folding_and_atom_rewrites() {
    let source = "func main(){var first=1+2*3;var second=first+(4*5);}";
    let (mut ast, _) = optimize(source, OptimizeConfig::default());
    assert_eq!(&source[ast.body[0].span().into_range()], source);
    let body = body_mut(&mut ast.body[0]);
    let Statement::VariableDecl(first) = &body[0] else {
        panic!()
    };
    assert!(matches!(&*first.init_val, Expression::ConstValueE(value, _) if value.value == "7"));
    assert_eq!(&source[first.init_val.span().into_range()], "1+2*3");
    let Statement::VariableDecl(second) = &body[1] else {
        panic!()
    };
    assert_eq!(&source[second.init_val.span().into_range()], "first+(4*5)");
    let Expression::CompositeE(composite, _) = &*second.init_val else {
        panic!()
    };
    assert_eq!(&source[composite.members[0].span().into_range()], "first");
    assert_eq!(&source[composite.members[1].span().into_range()], "(4*5)");
}
