//! Exhaustive user-facing messages: adding an error variant requires a message.
use super::*;
use std::fmt;

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolve(error) => error.fmt(f),
            Self::IllegalUse(error) => error.fmt(f),
        }
    }
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ResolveError::*;
        f.write_str(match self {
            UnknownEnum { name } => return write!(f, "Enum `{name}` is not in scope"),
            NotAnEnum { name } => return write!(f, "`{name}` is not an enum"),
            UnknownEnumVariant { owner, variant } => {
                return write!(f, "Enum `{owner}` has no variant `{variant}`");
            }
            UnknownType => "Unknown type",
            UnknownGeneric => "Unknown generic",
            UnknownConstraint => "Unknown generic constraint",
            MissingType => "Missing type; cannot infer it from the initializer",
            UnknownVariable => "Unknown variable",
            UnknownFunction => "Unknown function",
            UnknownInterface => "Unknown interface",
            Constraint(error) => return error.fmt(f),
        })
    }
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRequirement => "Invalid generic requirement",
            Self::InvalidArgument => "Invalid generic constraint argument",
            Self::NoRequirements => "Missing generic requirements",
            Self::NoArgument => "Missing generic constraint argument",
        })
    }
}

impl fmt::Display for CAbiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GenericUnit => "C ABI units cannot be generic",
            Self::GenericFunction => "C ABI functions cannot be generic",
            Self::UnitHasInterface => "C ABI units cannot define interfaces",
            Self::NonCAbiUnitParameter => "C ABI parameters cannot use non-C-ABI units",
        })
    }
}

impl fmt::Display for IllegalUseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IllegalUseError::*;
        f.write_str(match self {
            InvalidUnion { reason } => return f.write_str(reason),
            InvalidFunctionPointerOperation => "Function pointers support calling, copying and borrowing, not arithmetic or value matching",
            FunctionPointerRequiresInitializer => "Every function pointer array element must be initialized; null function pointers are not supported",
            ReservedDeconstruct => "Unit deconstruct is generated automatically and cannot be defined by the user",
            NonConstantMatchCase => "Match case values must be compile-time constants",
            MissingDefaultBranch => "Match requires a default branch: _ => { ... }",
            ResourceRequiresOwner => "A temporary resource requires an explicit res owner",
            ResourceInferenceRequiresInitializer => "Bare res is only allowed in a variable declaration with a resource initializer",
            ResourceUseAfterMove { name } => return write!(f, "Resource '{name}' has been moved or destroyed"),
            ResourceMoveWhileBorrowed { name } => return write!(f, "Cannot move or destroy resource '{name}' while it is borrowed"),
            ResourceEscapesScope => "A borrowed resource cannot outlive its owner",
            ResourceLoopMove => "Moving an outer resource inside a repeating loop is not supported",
            UnsupportedResourceOperation => "This operation is not supported for an owned resource",
            ResourceAggregateUnsupported => "Allocating resource-containing elements is not supported yet",
            EnumValueRequiresPrefix => "Enum values require an explicit prefix; use Enum::Variant or module::Enum::Variant",
            TypeMismatched { expected, found } => {
                return write!(f, "Type mismatch: expected {expected}, found {found}");
            }
            IntegerConstantOutOfRange { value, target } => return write!(f, "Integer constant {value} is out of range for {target}; truncation is not allowed"),
            NumericConstantLoss { target } => return write!(f, "Numeric constant cannot be represented by {target} without overflow or precision loss; use an explicit cast"),
            InvalidBuiltinArgument { name } => return write!(f, "Invalid argument or target type for builtin `{name}`"),
            DuplicateSymbol { name } => return write!(f, "Symbol `{name}` is already defined"),
            DuplicateVariable { name } => return write!(f, "Variable `{name}` is already defined; shadowing is not allowed"),
            CAbi(error) => return error.fmt(f),
            InvalidStringLiteral => "Invalid string literal or escape sequence",
            ArrayInitializerRequiresBrackets => "Array initializers must use '[' and ']', not '{' and '}'",
            InvalidExportTable => "Invalid export table",
            DuplicateFileId => "Duplicate translation unit file ID",
            CyclicUsing => "Cyclic using aliases",
            UnsupportedUsingTarget => "Unsupported using target",
            InvalidSelfBinding => "Invalid self binding",
            TypeUsedAsValue => "A type cannot be used as a value",
            UnsupportedSymbolValue => "Using this symbol as a value is not supported",
            SymbolNotCallable => "This symbol is not callable",
            GenericCountMismatch => "Generic argument count mismatch",
            GenericInferenceMissing { name } => return write!(f, "Cannot infer generic parameter `{name}` from the arguments; specify generic arguments explicitly"),
            GenericInferenceConflict { name, expected, found } => return write!(f, "Conflicting inferred types for generic parameter `{name}`: `{expected}` and `{found}`"),
            NonGenericInstantiation => "Cannot instantiate a non-generic symbol",
            PrivateInstantiation => "Cannot instantiate a non-exported symbol from another translation unit",
            RequirementUnmet { ty, requirement, reason } => return write!(f, "Type `{ty}` does not satisfy generic requirement `{requirement}`: {reason}"),
            MemberAccessViolation => "Invalid member access: check pub visibility and use -> for self",
            InvalidIndexAccess => "Indexing requires an array or reference and an integer index",
            InvalidDereference => "Only reference types can be dereferenced",
            UnsupportedGenericCall => "Unsupported generic call",
            IllegalVisibility => "This visibility modifier is not allowed on this declaration",
            CannotInferenceType => "Cannot infer the expression type",
            VariableMustBeInitialized => "Variable declarations require an initializer",
            NonConstantInitializer => "A const initializer must be a compile-time constant",
            ExpressionMustConditional => "A condition expression is required here",
            ReferenceCannotBeCalculated => "References cannot participate in this operation",
            InvalidAssignment => "The assignment target is not writable or is not an assignable location",
            LoopControlOutsideLoop => "break/continue can only be used inside a loop",
            ReturnOutsideFunction => "return can only be used inside a function or interface",
            ReturnValueRequired => "This function requires a return value",
            UnexpectedReturnValue => {
                "Cannot return a value without a declared return type; declare a return type or use return;"
            }
            ForBoundMustBeInteger => "for loop bounds must be integers",
            DuplicateDefaultBranch => "match can have only one default branch",
            ArgumentCountMismatch => "Argument count does not match the parameter list",
        })
    }
}
