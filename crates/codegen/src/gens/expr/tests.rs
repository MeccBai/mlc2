use super::*;
use crate::ast::{
    config::FileId,
    expression::{Composite, operators::Operator as AstOp},
    types::base_type::DataType,
};
use crate::gens::func::SymbolIr;
use crate::gens::instruction::{InstructionBuffer, Operator};

fn setup(signed: bool) -> (PackageSymbolTable, TypeIndex) {
    let mut package = PackageSymbolTable::new();
    let id = FileId::new(0);
    package.register(id).unwrap();
    let ty = package
        .file_mut(id)
        .unwrap()
        .get_base(DataType::Integer, 32, signed);
    (package, ty)
}
fn literal(ty: TypeIndex, text: &str) -> CompAtom {
    CompAtom::ConstValueA(ConstValue {
        value: text.into(),
        ty,
    })
}
fn expr(ty: TypeIndex, ops: Vec<AstOp>) -> Expression {
    Expression::CompositeE(Composite {
        members: ["20", "3", "2"].map(|v| literal(ty, v)).to_vec(),
        operators: ops,
    })
}
fn operations(value: &LlvmValue) -> Vec<Operator> {
    value
        .code
        .iter()
        .filter_map(|i| match i {
            Instruction::Operation { op, .. } => Some(*op),
            _ => None,
        })
        .collect()
}

#[test]
fn precedence_climbing_and_associativity() {
    let (package, ty) = setup(true);
    let (_, value) = IrGenerator::expression_expand(
        5,
        &expr(ty, vec![AstOp::Add, AstOp::Multiply]),
        &package,
        &HashMap::new(),
    );
    assert_eq!(operations(&value), vec![Operator::Mul, Operator::Add]);
    let (next, value) = IrGenerator::expression_expand(
        5,
        &expr(ty, vec![AstOp::Subtract, AstOp::Subtract]),
        &package,
        &HashMap::new(),
    );
    assert_eq!(operations(&value), vec![Operator::Sub, Operator::Sub]);
    assert_eq!(value.reg, Some(next - 1));
    assert!(value.in_reg);
    let targets: Vec<_> = value
        .code
        .iter()
        .filter_map(|i| match i {
            Instruction::Alloca { target, .. }
            | Instruction::Load { target, .. }
            | Instruction::Operation { target, .. } => Some(*target),
            _ => None,
        })
        .collect();
    assert_eq!(
        targets
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        targets.len()
    );
}

#[test]
fn grouping_and_unsigned_operations() {
    let (package, ty) = setup(false);
    let group = CompAtom::CompositeA(Composite {
        members: vec![literal(ty, "20"), literal(ty, "3")],
        operators: vec![AstOp::Add],
    });
    let expression = Expression::CompositeE(Composite {
        members: vec![group, literal(ty, "2")],
        operators: vec![AstOp::Divide],
    });
    let (_, value) = IrGenerator::expression_expand(0, &expression, &package, &HashMap::new());
    assert_eq!(operations(&value), vec![Operator::Add, Operator::UDiv]);
}

#[test]
fn variable_binding_is_loaded_without_replaying_initialization() {
    let (package, ty) = setup(true);
    let variable = std::rc::Rc::new(crate::ast::statement::Variable {
        read_count: Default::default(),
        declaration_span: (0..0).into(),
        name: "a".into(),
        var_type: ty,
        init_val: Box::new(Expression::Poison),
    });
    let mut bindings = HashMap::new();
    bindings.insert(
        "a".into(),
        LlvmValue {
            code: vec![Instruction::Unreachable],
            ty: LlvmType::Int(32),
            reg: Some(1),
            in_reg: false,
        },
    );
    let expression = Expression::CompositeE(Composite {
        members: vec![CompAtom::VarValueA(variable), literal(ty, "2")],
        operators: vec![AstOp::Add],
    });
    let (_, value) = IrGenerator::expression_expand(2, &expression, &package, &bindings);
    assert!(matches!(
        value.code[0],
        Instruction::Load {
            pointer: IrValue::Reg(1),
            ..
        }
    ));
    assert!(!value.code.contains(&Instruction::Unreachable));
}

#[test]
fn malformed_operands_and_overflow_are_collected() {
    let (package, ty) = setup(true);
    let bindings = HashMap::new();
    let malformed = Expression::CompositeE(Composite {
        members: vec![],
        operators: vec![],
    });
    let invalid_operands = expr(ty, vec![AstOp::LogicalAnd, AstOp::LogicalOr]);
    let literal = literal(ty, "1").to_expression();
    let mut handle = crate::gens::error::GenerationErrorHandle::default();
    for (start, expression) in [
        (0, &malformed),
        (0, &invalid_operands),
        (usize::MAX, &literal),
    ] {
        assert!(
            handle
                .collect("invalid expression", || {
                    IrGenerator::expression_expand(start, expression, &package, &bindings)
                })
                .is_none()
        );
    }
    let errors = handle.finish(()).unwrap_err();
    assert_eq!(errors.errors.len(), 3);
    assert!(
        errors
            .errors
            .iter()
            .all(|error| error.kind == crate::gens::error::GenerationErrorKind::Internal)
    );
}

#[test]
fn nested_short_circuit_and_float_pass_llvm_verification() {
    let (mut package, _) = setup(true);
    let bool_ty = package
        .file_mut(FileId::new(0))
        .unwrap()
        .get_base(DataType::Boolean, 8, false);
    let nested = CompAtom::CompositeA(Composite {
        members: vec![literal(bool_ty, "true"), literal(bool_ty, "false")],
        operators: vec![AstOp::LogicalOr],
    });
    let expression = Expression::CompositeE(Composite {
        members: vec![literal(bool_ty, "false"), nested],
        operators: vec![AstOp::LogicalAnd],
    });
    let (_, value) = IrGenerator::expression_expand(0, &expression, &package, &HashMap::new());
    assert_eq!(
        value
            .code
            .iter()
            .filter(|i| matches!(i, Instruction::ConditionalBranch { .. }))
            .count(),
        2
    );
    assert!(operations(&value).is_empty());
    let mut buffer = InstructionBuffer {
        instructions: value.code,
    };
    buffer.push(Instruction::Return {
        value: Some(TypedValue {
            ty: value.ty,
            value: IrValue::Reg(value.reg.unwrap()),
        }),
    });
    let bool_ir = format!("define i1 @logic() {{\n{}\n}}", buffer.format());
    let float_ty = package
        .file_mut(FileId::new(0))
        .unwrap()
        .get_base(DataType::Float, 32, true);
    let float_expr = Expression::CompositeE(Composite {
        members: vec![literal(float_ty, "1.25"), literal(float_ty, "2.5")],
        operators: vec![AstOp::Add],
    });
    let (_, value) = IrGenerator::expression_expand(0, &float_expr, &package, &HashMap::new());
    assert_eq!(operations(&value), vec![Operator::FAdd]);
    let mut buffer = InstructionBuffer {
        instructions: value.code,
    };
    buffer.push(Instruction::Return {
        value: Some(TypedValue {
            ty: value.ty,
            value: IrValue::Reg(value.reg.unwrap()),
        }),
    });
    let ir = format!(
        "{bool_ir}\ndefine float @floating() {{\n{}\n}}",
        buffer.format()
    );
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path = std::env::temp_dir().join(format!("mlc2-expr-control-{}.obj", std::process::id()));
    compiler.emit(ir, &path).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn generated_arithmetic_passes_llvm_verification() {
    let (package, ty) = setup(true);
    let (_, value) = IrGenerator::expression_expand(
        0,
        &expr(ty, vec![AstOp::Add, AstOp::Multiply]),
        &package,
        &HashMap::new(),
    );
    let result = value.reg.unwrap();
    let mut buffer = InstructionBuffer {
        instructions: value.code,
    };
    buffer.push(Instruction::Return {
        value: Some(TypedValue {
            ty: value.ty,
            value: IrValue::Reg(result),
        }),
    });
    let ir = format!("define i32 @main() {{\n{}\n}}", buffer.format());
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path = std::env::temp_dir().join(format!("mlc2-expr-{}.obj", std::process::id()));
    compiler.emit(ir, &path).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn array_index_and_call_pass_llvm_verification() {
    use crate::ast::{
        expression::{Access, FuncCall},
        function::FuncSymbol,
        symbols::EnumBool,
        types::ListType,
    };
    let (mut package, ty) = setup(true);
    let symbols = package.file_mut(FileId::new(0)).unwrap();
    let array_ty = symbols
        .types
        .insert("array".into(), CompileType::List(ListType::new(ty, 2)));
    let symbol = FuncSymbol {
        name: "f".into(),
        params: vec![(ty, "x".into())],
        ret_type: Some(ty),
        generics: vec![],
        generic_map: HashMap::new(),
        attributes: std::collections::HashSet::new(),
        exported: false,
    };
    let func = symbols.functions.insert("f".into(), symbol);
    let array = Expression::InitListE(InitialList::Array {
        ty: array_ty,
        values: vec![
            literal(ty, "3").to_expression(),
            literal(ty, "7").to_expression(),
        ],
    });
    let argument = Expression::UnaryExprE(UnaryExpr::Access(Access::Index {
        base: Box::new(array),
        index: Box::new(literal(ty, "1").to_expression()),
    }));
    let expression = Expression::FuncCallE(FuncCall {
        func: EnumBool::False(func),
        args: vec![argument],
    });
    let (_, value) = IrGenerator::expression_expand(0, &expression, &package, &HashMap::new());
    assert!(matches!(
        value.code.last(),
        Some(Instruction::MappedCall { .. })
    ));
    let mut buffer = InstructionBuffer {
        instructions: value.code,
    };
    buffer.push(Instruction::Return {
        value: Some(TypedValue {
            ty: value.ty,
            value: IrValue::Reg(value.reg.unwrap()),
        }),
    });
    let decl = package
        .get_function(func, false)
        .llvm_func(&package)
        .unwrap();
    let ir = format!("{decl}\ndefine i32 @main() {{\n{}\n}}", buffer.format());
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let path = std::env::temp_dir().join(format!("mlc2-expr-array-{}.obj", std::process::id()));
    compiler.emit(ir, &path).unwrap();
    std::fs::remove_file(path).unwrap();
}
