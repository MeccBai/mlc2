use super::*;
use crate::ast::{
    config::FileId,
    types::{ListType, base_type::DataType},
};
use crate::gens::instruction::TypedValue;

#[test]
fn call_allocates_result_ids_and_returns_structured_values() {
    let (package, small, large, _) = setup();
    let direct = symbol(vec![], Some(small)).llvm_func(&package).unwrap();
    let (next, value) = direct.call(10, &[]).unwrap();
    assert_eq!(next, 11);
    assert_eq!(value.reg, Some(10));
    assert_eq!(value.ty, LlvmType::Int(64));
    assert!(value.in_reg);
    assert!(matches!(
        &value.code[0],
        crate::gens::instruction::Instruction::MappedCall {
            target: Some(10),
            ..
        }
    ));

    let indirect = symbol(vec![], Some(large)).llvm_func(&package).unwrap();
    let (next, value) = indirect.call(next, &[]).unwrap();
    assert_eq!(next, 12);
    assert_eq!(value.reg, Some(11));
    assert!(!value.in_reg);
    assert_eq!(
        value.ty,
        LlvmType::Array {
            element: Box::new(LlvmType::Int(64)),
            length: 2
        }
    );
    assert!(matches!(
        &value.code[0],
        crate::gens::instruction::Instruction::Alloca { target: 11, .. }
    ));
    assert!(
        matches!(&value.code[1], crate::gens::instruction::Instruction::MappedCall { target: None, args, .. }
        if args[0].value == IrValue::Reg(11))
    );

    let void = symbol(vec![], None).llvm_func(&package).unwrap();
    let (after_void, value) = void.call(next, &[]).unwrap();
    assert_eq!(after_void, next);
    assert_eq!(value.reg, None);
    assert_eq!(value.ty, LlvmType::Void);
    assert_eq!(value.code.len(), 1);
    assert_eq!(
        direct.call(usize::MAX, &[]),
        Err(CallError::RegisterOverflow)
    );
}

#[test]
fn indirect_call_instructions_pass_llvm_verification() {
    let (package, _, large, _) = setup();
    let func = symbol(vec![], Some(large)).llvm_func(&package).unwrap();
    let (_, value) = func.call(4, &[]).unwrap();
    let buffer = crate::gens::instruction::InstructionBuffer {
        instructions: value.code,
    };
    let ir = format!(
        "{func}\ndefine i32 @main() {{\n{}  ret i32 0\n}}",
        buffer.format()
    );
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let output = std::env::temp_dir().join(format!("mlc2-call-value-{}.obj", std::process::id()));
    compiler.emit(ir, &output).unwrap();
    std::fs::remove_file(output).unwrap();
}
use std::collections::HashMap;

#[test]
fn symbol_produces_source_mappings_and_calls_use_converted_values() {
    let (package, small, large, immutable) = setup();
    let func = symbol(vec![small, immutable, large], Some(large));
    let lowered = func.llvm_func(&package).unwrap();
    assert_eq!(lowered.result().source, Some(large));
    assert_eq!(lowered.params()[1].source, immutable);
    let args = vec![
        TypedValue {
            ty: LlvmType::Ptr,
            value: IrValue::Reg(0),
        },
        TypedValue {
            ty: LlvmType::Int(64),
            value: IrValue::Integer(1),
        },
        TypedValue {
            ty: LlvmType::Ptr,
            value: IrValue::Reg(1),
        },
        TypedValue {
            ty: LlvmType::Ptr,
            value: IrValue::Reg(2),
        },
    ];
    assert_eq!(
        lowered.call_text(None, &args).unwrap(),
        "call void @\"unnamed::test\"(ptr sret([2 x i64]) align 8 %r0, i64 1, ptr readonly align 8 %r1, ptr byval([2 x i64]) align 8 %r2)"
    );
    assert!(matches!(
        lowered.call_text(Some(4), &args),
        Err(CallError::VoidResultTarget)
    ));
}

#[test]
fn split_mapping_flattens_into_multiple_call_arguments() {
    let (package, small, _, _) = setup();
    let mut lowered = symbol(vec![small], Some(small))
        .llvm_func(&package)
        .unwrap();
    let second = lowered.params[0].lowered[0].clone();
    lowered.params[0].lowered.push(second);
    let args = [
        TypedValue {
            ty: LlvmType::Int(64),
            value: IrValue::Integer(1),
        },
        TypedValue {
            ty: LlvmType::Int(64),
            value: IrValue::Integer(2),
        },
    ];
    assert_eq!(
        lowered.call_text(Some(3), &args).unwrap(),
        "%r3 = call i64 @\"unnamed::test\"(i64 1, i64 2)"
    );
    assert_eq!(
        lowered.to_string(),
        "declare i64 @\"unnamed::test\"(i64, i64)"
    );
}

#[test]
fn invalid_calls_do_not_write_partial_text() {
    let (package, small, _, _) = setup();
    let lowered = symbol(vec![small], Some(small))
        .llvm_func(&package)
        .unwrap();
    let mut output = String::from("prefix");
    assert!(matches!(
        lowered.write_call(&mut output, None, &[]),
        Err(CallError::ArgumentCount { .. })
    ));
    assert!(matches!(
        lowered.write_call(
            &mut output,
            None,
            &[TypedValue {
                ty: LlvmType::Ptr,
                value: IrValue::Null
            }]
        ),
        Err(CallError::ArgumentType { .. })
    ));
    assert_eq!(output, "prefix");
}

#[test]
fn variadic_calls_include_fixed_signature_and_extra_typed_values() {
    let (package, small, _, _) = setup();
    let mut func = symbol(vec![small], Some(small));
    func.params.push((TypeIndex::empty(), "...".into()));
    let lowered = func.llvm_func(&package).unwrap();
    let args = [
        TypedValue {
            ty: LlvmType::Int(64),
            value: IrValue::Integer(1),
        },
        TypedValue {
            ty: LlvmType::Double,
            value: IrValue::Val("1.0".into()),
        },
    ];
    assert_eq!(
        lowered.call_text(Some(1), &args).unwrap(),
        "%r1 = call i64 (i64, ...) @\"unnamed::test\"(i64 1, double 1.0)"
    );
}

fn setup() -> (PackageSymbolTable, TypeIndex, TypeIndex, TypeIndex) {
    let mut package = PackageSymbolTable::new();
    let id = FileId::new(31);
    package.register(id).unwrap();
    let symbols = package.file_mut(id).unwrap();
    let small = symbols.get_base(DataType::Integer, 64, true);
    let large = symbols
        .types
        .insert("large".into(), CompileType::List(ListType::new(small, 2)));
    let immutable = large.into(ValueType::Final, &mut symbols.types);
    (package, small, large, immutable)
}

fn symbol(params: Vec<TypeIndex>, ret: Option<TypeIndex>) -> FuncSymbol {
    FuncSymbol {
        name: "unnamed::test".into(),
        params: params
            .into_iter()
            .enumerate()
            .map(|(i, ty)| (ty, format!("p{i}")))
            .collect(),
        ret_type: ret,
        generics: vec![],
        generic_map: HashMap::new(),
        attributes: HashSet::new(),
        exported: false,
    }
}

#[test]
fn large_return_precedes_readonly_and_copy_parameters() {
    let (package, small, large, immutable) = setup();
    let func = symbol(vec![small, immutable, large], Some(large));
    let abi = LlvmFunc::function(&func, &package).unwrap();
    assert_eq!(abi.result.ty, LlvmType::Void);
    assert_eq!(
        abi.parameters().map(|p| p.mode).collect::<Vec<_>>(),
        [
            ParameterMode::ReturnPointer,
            ParameterMode::Direct,
            ParameterMode::ReadOnlyPointer,
            ParameterMode::CopyPointer
        ]
    );
    assert_eq!(
        abi.to_string(),
        "declare void @\"unnamed::test\"(ptr sret([2 x i64]) align 8, i64, ptr readonly align 8, ptr byval([2 x i64]) align 8)"
    );
}

#[test]
fn eight_bytes_stays_direct_and_declarations_are_deduplicated() {
    let (package, small, _, _) = setup();
    let func = symbol(vec![small], Some(small));
    let mut generator = IrGenerator::new(crate::manifest::X86_64_PC_WINDOWS_MSVC.into());
    generator.add_function_decl(&func, &package);
    generator.add_function_decl(&func, &package);
    assert_eq!(generator.defines, "declare i64 @\"unnamed::test\"(i64)\n");
}

#[test]
fn variadic_marker_is_not_lowered_as_a_type() {
    let (package, small, _, _) = setup();
    let mut func = symbol(vec![small], None);
    func.params.push((TypeIndex::empty(), "...".into()));
    assert_eq!(
        LlvmFunc::function(&func, &package).unwrap().to_string(),
        "declare void @\"unnamed::test\"(i64, ...)"
    );
}

#[test]
fn c_abi_uses_original_name_and_does_not_use_internal_aggregate_abi() {
    let (package, small, large, _) = setup();
    let mut func = symbol(vec![small], Some(small));
    func.attributes.insert(FuncAttibute::Cabi);
    assert_eq!(LlvmFunc::function(&func, &package).unwrap().name, "test");
    func.params[0].0 = large;
    assert_eq!(
        LlvmFunc::function(&func, &package),
        Err(DeclarationError::UnsupportedCAbiAggregate)
    );
}

#[test]
fn large_return_then_receiver_then_explicit_parameters() {
    let (package, small, large, _) = setup();
    let func = InterfaceSymbol {
        name: "Owner::method".into(),
        public: true,
        has_self: true,
        mutable: true,
        exported: false,
        owner: large,
        attributes: HashSet::new(),
        generics: vec![],
        generic_map: HashMap::new(),
        params: vec![(small, "p".into())],
        ret_type: Some(large),
    };
    let abi = LlvmFunc::interface(&func, &package).unwrap();
    assert_eq!(
        abi.parameters().map(|p| p.mode).collect::<Vec<_>>(),
        [
            ParameterMode::ReturnPointer,
            ParameterMode::Receiver,
            ParameterMode::Direct
        ]
    );
}

#[test]
fn llvm_accepts_generated_declaration_attributes() {
    let (package, small, large, immutable) = setup();
    let abi = LlvmFunc::function(
        &symbol(vec![small, immutable, large], Some(large)),
        &package,
    )
    .unwrap();
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let output = std::env::temp_dir().join(format!("mlc2-declaration-{}.obj", std::process::id()));
    compiler
        .emit(
            format!("{abi}\ndefine i32 @main() {{ ret i32 0 }}"),
            &output,
        )
        .unwrap();
    std::fs::remove_file(output).unwrap();
}
