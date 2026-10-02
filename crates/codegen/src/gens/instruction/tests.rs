use super::*;

fn integer(value: i128) -> TypedValue {
    TypedValue {
        ty: LlvmType::Int(32),
        value: IrValue::Integer(value),
    }
}

#[test]
fn batch_render_appends_to_existing_buffer() {
    let mut buffer = InstructionBuffer::default();
    buffer.push(Instruction::BasicBlock {
        label: "entry".into(),
    });
    buffer.push(Instruction::Alloca {
        target: 0,
        ty: LlvmType::Int(32),
        count: None,
        align: Some(4),
    });
    buffer.push(Instruction::Store {
        pointer: IrValue::Reg(0),
        value: integer(7),
        align: Some(4),
    });
    buffer.push(Instruction::Load {
        target: 1,
        ty: LlvmType::Int(32),
        pointer: IrValue::Reg(0),
        align: Some(4),
    });
    buffer.push(Instruction::Return {
        value: Some(TypedValue {
            ty: LlvmType::Int(32),
            value: IrValue::Reg(1),
        }),
    });
    let mut text = String::from("define i32 @main() {\n");
    buffer.write_to(&mut text).unwrap();
    text.push_str("}\n");
    assert!(text.contains("%r0 = alloca i32, align 4"));
    let compiler = crate::llvm::IrCompiler::init(crate::manifest::X86_64_PC_WINDOWS_MSVC).unwrap();
    let output = std::env::temp_dir().join(format!("mlc2-instruction-{}.obj", std::process::id()));
    compiler.emit(text, &output).unwrap();
    assert!(std::fs::metadata(&output).unwrap().len() > 0);
    std::fs::remove_file(output).unwrap();
}

#[test]
fn call_uses_typed_arguments_and_supports_void_and_variadic_calls() {
    let call = Instruction::Call {
        target: None,
        func: IrValue::Global("print".into()),
        return_type: LlvmType::Void,
        args: vec![integer(1)],
        variadic_params: None,
    };
    assert_eq!(call.to_string(), "  call void @\"print\"(i32 1)");
    let call = Instruction::Call {
        target: Some(3),
        func: IrValue::Global("printf".into()),
        return_type: LlvmType::Int(32),
        args: vec![
            TypedValue {
                ty: LlvmType::Ptr,
                value: IrValue::Null,
            },
            integer(1),
        ],
        variadic_params: Some(vec![LlvmType::Ptr]),
    };
    assert_eq!(
        call.to_string(),
        "  %r3 = call i32 (ptr, ...) @\"printf\"(ptr null, i32 1)"
    );
}

#[test]
fn gep_supports_multiple_typed_indices() {
    let gep = Instruction::Gep {
        target: 4,
        ty: LlvmType::Array {
            element: Box::new(LlvmType::Int(32)),
            length: 3,
        },
        pointer: IrValue::Reg(0),
        indices: vec![integer(0), integer(2)],
        inbounds: true,
    };
    assert_eq!(
        gep.to_string(),
        "  %r4 = getelementptr inbounds [3 x i32], ptr %r0, i32 0, i32 2"
    );
}

#[test]
fn identifiers_escape_llvm_bytes() {
    assert_eq!(
        IrValue::Global("a\"\\中".into()).to_string(),
        "@\"a\\22\\5C\\E4\\B8\\AD\""
    );
    assert_eq!(
        LlvmType::Named("module::Point".into()).to_string(),
        "%\"module::Point\""
    );
}

#[test]
fn control_flow_and_phi_render_consistent_labels() {
    let branch = Instruction::ConditionalBranch {
        condition: IrValue::Reg(1),
        then_label: "yes".into(),
        else_label: "no".into(),
    };
    assert_eq!(
        branch.to_string(),
        "  br i1 %r1, label %\"yes\", label %\"no\""
    );
    let phi = Instruction::Phi {
        target: 3,
        ty: LlvmType::Int(32),
        incoming: vec![
            (IrValue::Integer(1), "yes".into()),
            (IrValue::Integer(2), "no".into()),
        ],
    };
    assert_eq!(
        phi.to_string(),
        "  %r3 = phi i32 [ 1, %\"yes\" ], [ 2, %\"no\" ]"
    );
    assert_eq!(
        Instruction::Return { value: None }.to_string(),
        "  ret void"
    );
}

#[test]
fn signed_float_and_cast_operations_have_distinct_spelling() {
    assert_eq!(Operator::SDiv.to_string(), "sdiv");
    assert_eq!(Operator::UDiv.to_string(), "udiv");
    assert_eq!(FloatCondition::ONe.to_string(), "one");
    assert_eq!(Condition::ULt.to_string(), "ult");
    assert_eq!(
        Instruction::Cast {
            target: 5,
            op: Cast::SIToFP,
            value: integer(-1),
            to: LlvmType::Double
        }
        .to_string(),
        "  %r5 = sitofp i32 -1 to double"
    );
}
