use super::*;
use crate::ast::{
    AbstractSyntaxTree, GenericIndex, SymbolTable,
    config::FileId,
    expression::{
        CompAtom, Composite, ConstValue, FuncCall, InitialList, UnaryExpr, operators::Operator,
    },
    function::FuncSymbol,
    statement::Variable,
    symbols::{EnumBool, PackageSymbolTable},
    types::{ValueType, base_type::DataType},
};
use crate::diagnostic::{
    error::{CompileError, ErrorHandle, IllegalUseError},
    warning::WarningHandle,
};
use crate::parser::out::{TempPath, TempType};
use std::{collections::HashMap, rc::Rc};

fn config() -> Config {
    Config::new(
        FileId::new(0),
        vec![],
        String::new(),
        "infer.m2".into(),
        ErrorHandle::new("infer.m2".into()),
        WarningHandle::new("infer.m2".into()),
    )
}

fn literal(value: &str, symbols: &SymbolTable) -> Expression {
    Expression::ConstValueE(ConstValue {
        value: value.into(),
        ty: symbols.get_base(DataType::Integer, 32, true),
    }, (0..0).into())
}

fn variable(ty: TypeIndex) -> Expression {
    Expression::VarValueE(Rc::new(Variable {
        read_count: Default::default(),
        declaration_span: (0..1).into(),
        name: "value".into(),
        var_type: ty,
        init_val: Box::new(Expression::null()),
    }), (0..0).into())
}

#[test]
fn expression_inference_reports_value_place_and_reference_mutability() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let ty = symbols.get_base(DataType::Integer, 32, true);
    for mutable in [false, true] {
        let reference = ty.make_ref(&mut symbols, mutable);
        let pointer = variable(reference.into_value_type(ValueType::Final, &mut symbols));
        let dereference = Expression::UnaryExprE(UnaryExpr::Operator {
            op: Operator::Dereference,
            value: Box::new(CompAtom::from_expr(pointer)),
        }, (0..0).into());
        let Inference::Value(result) =
            TypeInfer::new(&mut config, &mut symbols).infer(&dereference)
        else {
            panic!()
        };
        assert_eq!(result.category, ValueCategory::Place { mutable });
        assert_eq!(
            result.ty.value_type(&symbols),
            if mutable {
                ValueType::Flex
            } else {
                ValueType::Final
            }
        );
    }
    let value = literal("42", &symbols);
    let Inference::Value(result) = TypeInfer::new(&mut config, &mut symbols).infer(&value) else {
        panic!()
    };
    assert_eq!(result.category, ValueCategory::Value);
    assert_eq!(result.ty.value_type(&symbols), ValueType::Constant);
}

#[test]
fn readonly_constant_session_handles_folding_normalization_and_range_diagnostics() {
    let mut config = config();
    let symbols = SymbolTable::new();
    let ty = symbols.get_base(DataType::Integer, 8, false);
    let expression = Expression::CompositeE(Composite {
        members: vec![
            CompAtom::from_expr(literal("100", &symbols)),
            CompAtom::from_expr(literal("200", &symbols)),
        ],
        operators: vec![Operator::Add],
    }, (0..0).into());
    let mut infer = TypeInfer::read_only(&mut config, &symbols);
    assert!(infer.is_const(&expression));
    assert!(
        matches!(infer.constant(expression.clone()), Expression::ConstValueE(value, ..) if value.value=="300")
    );
    let span = (4..12).into();
    infer.check_constant_range(&expression, ty, span);
    assert_eq!(config.error_handle().errors.len(), 1);
    assert!(
        config
            .error_handle()
            .errors
            .contains(&crate::diagnostic::error::ErrorInfo::new(
                CompileError::IllegalUse(IllegalUseError::IntegerConstantOutOfRange {
                    value: "300".into(),
                    target: ty.format(&symbols)
                }),
                span,
            ))
    );
}

#[test]
fn check_modes_keep_initialization_numeric_rules_separate_from_arguments() {
    let mut symbols = SymbolTable::new();
    let target = symbols.get_base(DataType::Integer, 8, false);
    let value = literal("100", &symbols);
    assert!(TypeInfer::new(&mut config(), &mut symbols).check(
        &value,
        target,
        CheckMode::Initialization,
        (0..3).into()
    ));
    let mut config = config();
    assert!(!TypeInfer::new(&mut config, &mut symbols).check(
        &value,
        target,
        CheckMode::Argument,
        (0..3).into()
    ));
    assert_eq!(config.error_handle().errors.len(), 1);
}

#[test]
fn void_poison_and_target_typed_initializers_are_distinct() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let signature = crate::ast::types::FunctionType::declare(vec![], None, false, &mut symbols);
    let crate::ast::types::CompileType::Function(function) =
        symbols.types.get(signature).unqualified()
    else {
        panic!()
    };
    let call = Expression::FuncCallE(FuncCall { pipeline: false,
        callee: None,
        func: EnumBool::False(function.signature),
        args: vec![],
    }, (0..0).into());
    let mut infer = TypeInfer::new(&mut config, &mut symbols);
    assert_eq!(infer.infer(&call), Inference::Void);
    assert_eq!(infer.infer(&Expression::Poison((0..0).into())), Inference::Poison);
    assert_eq!(
        infer.infer(&Expression::InitListE(InitialList::List {
            onwer: None,
            values: vec![]
        }, (0..0).into())),
        Inference::NeedsExpectedType
    );
}

fn generic_function() -> (Config, PackageSymbolTable, FuncSymbol) {
    let source = "func<T> pair(a:T,b:T)->T{return a;}func main(){}";
    let tokens = crate::lexer::tokenize(source).unwrap();
    let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
    assert!(errors.is_empty());
    let mut ast = AbstractSyntaxTree::new(config(), module.unwrap());
    let mut package = PackageSymbolTable::new();
    ast.analysis(&mut package);
    assert!(!ast.config.is_poisoned());
    let symbol = package
        .file(ast.config.file_id())
        .unwrap()
        .generics
        .functions
        .entries()
        .next()
        .unwrap()
        .2
        .clone();
    (ast.config, package, symbol)
}

#[test]
fn generic_bindings_are_fresh_per_call_and_context_resolves_annotations() {
    let (mut config, mut package, symbol) = generic_function();
    let id = config.file_id();
    let mut symbols = package.resolve_context(id);
    let i32_type = symbols.local().get_base(DataType::Integer, 32, true);
    let f64_type = symbols.local().get_base(DataType::Float, 64, true);
    let mut infer = TypeInfer::new(&mut config, &mut symbols);
    let first = infer
        .generic_arguments(
            &symbol,
            &[variable(i32_type), variable(i32_type)],
            (0..1).into(),
        )
        .unwrap();
    let second = infer
        .generic_arguments(
            &symbol,
            &[variable(f64_type), variable(f64_type)],
            (0..1).into(),
        )
        .unwrap();
    let generic = symbol.generic_map["T"];
    assert_ne!(first[&generic], second[&generic]);
    let bindings = HashMap::from([("T".to_owned(), i32_type)]);
    let mut infer = TypeInfer::new(&mut config, &mut symbols).with_context(&bindings);
    assert_eq!(
        infer.resolve_type((
            TempType::Path(TempPath {
                segments: vec!["T".into()]
            }),
            (0..1).into()
        )),
        Some(i32_type)
    );
}

#[test]
fn generic_conflicts_submit_once_and_stop_poisoned_inference() {
    let (mut config, mut package, symbol) = generic_function();
    let id = config.file_id();
    let mut symbols = package.resolve_context(id);
    let i32_type = symbols.local().get_base(DataType::Integer, 32, true);
    let f64_type = symbols.local().get_base(DataType::Float, 64, true);
    let arguments = [variable(i32_type), variable(f64_type)];
    assert!(
        TypeInfer::new(&mut config, &mut symbols)
            .generic_arguments(&symbol, &arguments, (3..9).into())
            .is_none()
    );
    assert_eq!(config.error_handle().errors.len(), 1);
    assert_eq!(
        TypeInfer::new(&mut config, &mut symbols).infer(&arguments[0]),
        Inference::Poison
    );
    assert_eq!(config.error_handle().errors.len(), 1);
}
