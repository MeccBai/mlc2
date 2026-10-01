use super::*;
use crate::ast::symbols::EnumBool;
use crate::ast::{
    AnalyzedAst, expression::Expression, statement::Statement, types::base_type::DataType,
};
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use std::{collections::HashMap, rc::Rc};

fn config() -> Config {
    Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    )
}

fn compile(source: &str) -> AnalyzedAst {
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty(), "{errors:?}");
    let ast = AnalyzedAst::new(config(), module.unwrap());
    assert!(
        ast.config.error_handle().errors.is_empty(),
        "{:?}",
        ast.config.error_handle().errors
    );
    ast
}

#[test]
fn recursive_generic_body_reuses_registered_instance() {
    let ast = compile(
        "func<T> recur(value:T) -> T { return recur<T>(value); } func main() { var a = recur<i32>(1); }",
    );
    assert_eq!(ast.symbols.function_instances.len(), 1);
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let Statement::ReturnBlock(ret) = &instance.body[0] else {
        panic!("return expected");
    };
    let Some(Expression::FuncCallE(call)) = &ret.value else {
        panic!("call expected");
    };
    assert_eq!(call.func, EnumBool::False(instance.symbol));
}

#[test]
fn nested_generics_keep_distinct_bindings_and_mutual_recursion() {
    let ast = compile(
        "func<T> first(value:T) -> T { return second<T>(value); } func<T> second(value:T) -> T { return first<T>(value); } func main() { var a = first<i32>(1); }",
    );
    assert_eq!(ast.symbols.function_instances.len(), 2);
    let ast = compile(
        "generic Number { std::generic::is_integer; }; func<T:Number,U:Number> select(a:T,b:U) -> U { return b; } func main() { var a:i32 = 1; var b:u8 = 2; var result = select<i32,u8>(a,b); }",
    );
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let symbol = ast.symbols.functions.get(instance.symbol);
    assert_eq!(symbol.params[0].0.format(&ast.symbols.types), "i32");
    assert_eq!(symbol.params[1].0.format(&ast.symbols.types), "u8");
}

#[test]
fn concrete_local_declaration_and_use_share_rc() {
    let ast = compile(
        "func<T> identity(value:T) -> T { var local:T = value; return local; } func main() { var a = identity<i32>(1); var b = identity<i32>(2); }",
    );
    assert_eq!(ast.symbols.function_instances.len(), 1);
    let instance = ast.symbols.function_instances.values().next().unwrap();
    let Statement::VariableDecl(local) = &instance.body[0] else {
        panic!("local expected");
    };
    let Statement::ReturnBlock(ret) = &instance.body[1] else {
        panic!("return expected");
    };
    let Some(Expression::VarValueE(value)) = &ret.value else {
        panic!("variable expected");
    };
    assert!(Rc::ptr_eq(local, value));
    assert_eq!(local.var_type.format(&ast.symbols.types), "i32");
}

#[test]
fn interface_body_uses_new_symbol_and_concrete_parameter_variables() {
    let mut ast = compile(
        "unit Owner {}; Owner::func<T> method(value:T) -> T { var local:T = value; return local; } func main() {}",
    );
    let template = ast
        .symbols
        .generics
        .interfaces
        .get_by_name(&ast.config.symbol_name("Owner::method"))
        .unwrap();
    let generic = ast.symbols.generics.interfaces.get(template).generic_map["T"];
    let concrete = ast.symbols.get_base(DataType::Integer, 32, true);
    let actives = InstantiationActives::default();
    let params = HashMap::from([(generic, concrete)]);
    let instance = template.instantiation(
        &mut ast.config,
        &params,
        &mut ast.symbols,
        Some(&actives),
        (0..1).into(),
    );
    assert!(!instance.is_empty());
    assert!(actives.borrow().is_empty());
    assert!(ast.config.error_handle().errors.is_empty());
    let symbol = ast.symbols.interfaces.get(instance);
    assert_eq!(symbol.params[0].0, concrete);
    assert_eq!(symbol.ret_type, Some(concrete));
    assert!(symbol.generics.is_empty());
    let body = &ast.symbols.interface_instances[&instance];
    assert_eq!(body.symbol, instance);
    let Statement::VariableDecl(local) = &body.body[0] else {
        panic!("local expected");
    };
    assert_eq!(local.var_type, concrete);
    let Expression::VarValueE(parameter) = &*local.init_val else {
        panic!("parameter expected");
    };
    assert_eq!(parameter.name, "value");
    assert_eq!(parameter.var_type, symbol.params[0].0);
    let Statement::ReturnBlock(ret) = &body.body[1] else {
        panic!("return expected");
    };
    let Some(Expression::VarValueE(value)) = &ret.value else {
        panic!("variable expected");
    };
    assert!(Rc::ptr_eq(local, value));
    assert_eq!(
        template.instantiation(
            &mut ast.config,
            &params,
            &mut ast.symbols,
            Some(&actives),
            (0..1).into()
        ),
        instance
    );
    let original = ast.symbols.generics.interfaces.get(template);
    assert_eq!(original.generics, ["T"]);
    assert!(original.params[0].0.is_generic(&ast.symbols.types));
}

#[test]
fn function_body_parameters_belong_to_the_new_symbol_not_the_template() {
    let ast = compile(
        "func<T> identity(value:T) -> T { return value; } func main() { var number = identity<i32>(1); var boolean = identity<bool>(true); }",
    );
    let template = ast
        .symbols
        .generics
        .functions
        .get_by_name(&"identity".into())
        .unwrap();
    let original = ast.symbols.generics.functions.get(template);
    assert_eq!(original.generics, ["T"]);
    assert!(original.params[0].0.is_generic(&ast.symbols.types));
    let mut variables = Vec::new();
    for (index, instance) in &ast.symbols.function_instances {
        assert_eq!(*index, instance.symbol);
        let symbol = ast.symbols.functions.get(*index);
        assert!(symbol.generic_map.is_empty());
        let Statement::ReturnBlock(ret) = &instance.body[0] else {
            panic!("return expected");
        };
        let Some(Expression::VarValueE(parameter)) = &ret.value else {
            panic!("parameter expected");
        };
        assert_eq!(parameter.var_type, symbol.params[0].0);
        assert_eq!(symbol.ret_type, Some(parameter.var_type));
        variables.push(parameter.clone());
    }
    assert_eq!(variables.len(), 2);
    assert!(!Rc::ptr_eq(&variables[0], &variables[1]));
}

#[test]
fn body_error_checks_concrete_return_type_and_cleans_actives() {
    let mut ast = compile("func<T> wrong(value:T) -> T { return true; } func main() {}");
    let template = ast
        .symbols
        .generics
        .functions
        .get_by_name(&"wrong".into())
        .unwrap();
    let generic = ast.symbols.generics.functions.get(template).generic_map["T"];
    let concrete = ast.symbols.get_base(DataType::Integer, 32, true);
    let actives = InstantiationActives::default();
    let instance = template.instantiation(
        &mut ast.config,
        &HashMap::from([(generic, concrete)]),
        &mut ast.symbols,
        Some(&actives),
        (0..1).into(),
    );
    assert!(instance.is_empty());
    assert!(ast.config.is_poisoned());
    assert_eq!(ast.config.error_handle().errors.len(), 1);
    assert!(ast.symbols.function_instances.is_empty());
    assert!(actives.borrow().is_empty());
}
