use super::*;

/// LLVM quoted identifiers escape bytes, rather than using Rust string escapes.
fn identifier(f: &mut fmt::Formatter<'_>, name: &str) -> fmt::Result {
    f.write_str("\"")?;
    for byte in name.bytes() {
        if (32..=126).contains(&byte) && byte != b'"' && byte != b'\\' {
            write!(f, "{}", byte as char)?;
        } else {
            write!(f, "\\{byte:02X}")?;
        }
    }
    f.write_str("\"")
}

fn separated<T: fmt::Display>(f: &mut fmt::Formatter<'_>, items: &[T]) -> fmt::Result {
    for (i, item) in items.iter().enumerate() {
        if i != 0 {
            f.write_str(", ")?;
        }
        write!(f, "{item}")?;
    }
    Ok(())
}

impl fmt::Display for LlvmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => f.write_str("void"),
            Self::Int(bits) => write!(f, "i{bits}"),
            Self::Float => f.write_str("float"),
            Self::Double => f.write_str("double"),
            Self::Ptr => f.write_str("ptr"),
            Self::Array { element, length } => write!(f, "[{length} x {element}]"),
            Self::Struct(fields) => {
                f.write_str("{ ")?;
                separated(f, fields)?;
                f.write_str(" }")
            }
            Self::Named(name) => {
                f.write_str("%")?;
                identifier(f, name)
            }
        }
    }
}

impl fmt::Display for IrValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reg(index) => write!(f, "%r{index}"),
            Self::Global(name) => {
                f.write_str("@")?;
                identifier(f, name)
            }
            Self::Integer(value) => write!(f, "{value}"),
            Self::Bool(value) => f.write_str(if *value { "true" } else { "false" }),
            Self::Null => f.write_str("null"),
            Self::ZeroInitializer => f.write_str("zeroinitializer"),
            Self::Val(text) => f.write_str(text),
        }
    }
}

impl fmt::Display for TypedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.ty, self.value)
    }
}

macro_rules! spelling {
    ($ty:ty, $($variant:ident => $name:literal),* $(,)?) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(match self { $(Self::$variant => $name),* })
            }
        }
    };
}
spelling!(Operator, Add=>"add", Sub=>"sub", Mul=>"mul", SDiv=>"sdiv", UDiv=>"udiv", SRem=>"srem", URem=>"urem", FAdd=>"fadd", FSub=>"fsub", FMul=>"fmul", FDiv=>"fdiv", FRem=>"frem", And=>"and", Or=>"or", Xor=>"xor", Shl=>"shl", LShr=>"lshr", AShr=>"ashr");
spelling!(Condition, Eq=>"eq", Ne=>"ne", SLt=>"slt", SLe=>"sle", SGt=>"sgt", SGe=>"sge", ULt=>"ult", ULe=>"ule", UGt=>"ugt", UGe=>"uge");
spelling!(FloatCondition, OEq=>"oeq", ONe=>"one", OLt=>"olt", OLe=>"ole", OGt=>"ogt", OGe=>"oge", UEq=>"ueq", UNe=>"une", ULt=>"ult", ULe=>"ule", UGt=>"ugt", UGe=>"uge", Ord=>"ord", Uno=>"uno");
spelling!(Cast, Trunc=>"trunc", ZExt=>"zext", SExt=>"sext", FPTrunc=>"fptrunc", FPExt=>"fpext", FPToUI=>"fptoui", FPToSI=>"fptosi", UIToFP=>"uitofp", SIToFP=>"sitofp", PtrToInt=>"ptrtoint", IntToPtr=>"inttoptr", Bitcast=>"bitcast");

fn alignment(f: &mut fmt::Formatter<'_>, align: Option<usize>) -> fmt::Result {
    if let Some(align) = align {
        write!(f, ", align {align}")?;
    }
    Ok(())
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::BasicBlock { label } = self {
            identifier(f, label)?;
            return f.write_str(":");
        }
        f.write_str("  ")?;
        match self {
            Self::StringConstant { name, bytes } => {
                write!(
                    f,
                    "{} = private unnamed_addr constant [{} x i8] c\"",
                    IrValue::Global(name.clone()),
                    bytes.len()
                )?;
                for byte in bytes {
                    write!(f, "\\{byte:02X}")?;
                }
                f.write_str("\", align 1")
            }
            Self::MappedCall { func, target, args } => {
                func.write_call(f, *target, args).map_err(|_| fmt::Error)
            }
            Self::Switch { value, default, cases } => {
                write!(f, "switch {value}, label %{default} [")?;
                for (case, label) in cases { write!(f, "\n    {} {case}, label %{label}", value.ty)?; }
                write!(f, "\n  ]")
            }
            Self::IndirectCall {
                func,
                callee,
                target,
                args,
            } => func
                .write_call_to(f, *target, args, callee)
                .map_err(|_| fmt::Error),
            Self::FunctionAddress { func, target } => {
                write!(
                    f,
                    "%r{target} = bitcast ptr {} to ptr",
                    IrValue::Global(func.name().into())
                )
            }
            Self::Alloca {
                target,
                ty,
                count,
                align,
            } => {
                write!(f, "%r{target} = alloca {ty}")?;
                if let Some(count) = count {
                    write!(f, ", {count}")?;
                }
                alignment(f, *align)
            }
            Self::Store {
                pointer,
                value,
                align,
            } => {
                write!(f, "store {value}, ptr {pointer}")?;
                alignment(f, *align)
            }
            Self::Load {
                target,
                ty,
                pointer,
                align,
            } => {
                write!(f, "%r{target} = load {ty}, ptr {pointer}")?;
                alignment(f, *align)
            }
            Self::Gep {
                target,
                ty,
                pointer,
                indices,
                inbounds,
            } => {
                write!(
                    f,
                    "%r{target} = getelementptr {}{ty}, ptr {pointer}",
                    if *inbounds { "inbounds " } else { "" }
                )?;
                for index in indices {
                    write!(f, ", {index}")?;
                }
                Ok(())
            }
            Self::Call {
                target,
                func,
                return_type,
                args,
                variadic_params,
            } => {
                if let Some(target) = target {
                    write!(f, "%r{target} = ")?;
                }
                write!(f, "call {return_type} ")?;
                if let Some(params) = variadic_params {
                    f.write_str("(")?;
                    separated(f, params)?;
                    if !params.is_empty() {
                        f.write_str(", ")?;
                    }
                    f.write_str("...) ")?;
                }
                write!(f, "{func}(")?;
                separated(f, args)?;
                f.write_str(")")
            }
            Self::Operation {
                target,
                op,
                ty,
                lhs,
                rhs,
            } => write!(f, "%r{target} = {op} {ty} {lhs}, {rhs}"),
            Self::Compare {
                target,
                condition,
                ty,
                lhs,
                rhs,
            } => write!(f, "%r{target} = icmp {condition} {ty} {lhs}, {rhs}"),
            Self::FloatCompare {
                target,
                condition,
                ty,
                lhs,
                rhs,
            } => write!(f, "%r{target} = fcmp {condition} {ty} {lhs}, {rhs}"),
            Self::Cast {
                target,
                op,
                value,
                to,
            } => write!(f, "%r{target} = {op} {value} to {to}"),
            Self::Phi {
                target,
                ty,
                incoming,
            } => {
                write!(f, "%r{target} = phi {ty} ")?;
                for (i, (value, label)) in incoming.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "[ {value}, %")?;
                    identifier(f, label)?;
                    f.write_str(" ]")?;
                }
                Ok(())
            }
            Self::ExtractValue {
                target,
                aggregate,
                index,
            } => write!(f, "%r{target} = extractvalue {aggregate}, {index}"),
            Self::InsertValue {
                target,
                aggregate,
                value,
                index,
            } => write!(f, "%r{target} = insertvalue {aggregate}, {value}, {index}"),
            Self::Branch { label } => {
                f.write_str("br label %")?;
                identifier(f, label)
            }
            Self::ConditionalBranch {
                condition,
                then_label,
                else_label,
            } => {
                write!(f, "br i1 {condition}, label %")?;
                identifier(f, then_label)?;
                f.write_str(", label %")?;
                identifier(f, else_label)
            }
            Self::Return { value: Some(value) } => write!(f, "ret {value}"),
            Self::Return { value: None } => f.write_str("ret void"),
            Self::Unreachable => f.write_str("unreachable"),
            Self::BasicBlock { .. } => unreachable!(),
        }
    }
}
