use crate::lexer::TokenError;
use crate::parser::ParseError;
use crate::parser::out::Span;
use std::collections::HashSet;

use std::ops::Range;
mod messages;
mod render;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintError {
    InvalidRequirement,
    InvalidArgument,
    NoRequirements,
    NoArgument,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompileError {
    Resolve(ResolveError),
    IllegalUse(IllegalUseError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolveError {
    UnknownEnum { name: String },
    NotAnEnum { name: String },
    UnknownEnumVariant { owner: String, variant: String },
    UnknownType,
    UnknownGeneric,
    UnknownConstraint,
    MissingType,
    UnknownVariable,
    UnknownFunction,
    UnknownInterface,
    Constraint(ConstraintError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IllegalUseError {
    NonConstantMatchCase,
    MissingDefaultBranch,
    ResourceRequiresOwner,
    ResourceInferenceRequiresInitializer,
    ResourceUseAfterMove {
        name: String,
    },
    ResourceMoveWhileBorrowed {
        name: String,
    },
    ResourceEscapesScope,
    ResourceLoopMove,
    UnsupportedResourceOperation,
    ResourceAggregateUnsupported,
    EnumValueRequiresPrefix,
    InvalidStringLiteral,
    InvalidBuiltinArgument {
        name: String,
    },
    IntegerConstantOutOfRange {
        value: String,
        target: String,
    },
    NumericConstantLoss {
        target: String,
    },
    ArrayInitializerRequiresBrackets,
    CAbi(CAbiError),
    InvalidExportTable,
    DuplicateFileId,
    DuplicateSymbol {
        name: String,
    },
    CyclicUsing,
    UnsupportedUsingTarget,
    ReservedDeconstruct,
    InvalidUnion { reason: String },
    InvalidFunctionPointerOperation,
    FunctionPointerRequiresInitializer,
    DuplicateVariable {
        name: String,
    },
    InvalidSelfBinding,
    TypeUsedAsValue,
    UnsupportedSymbolValue,
    SymbolNotCallable,
    GenericCountMismatch,
    GenericInferenceMissing {
        name: String,
    },
    GenericInferenceConflict {
        name: String,
        expected: String,
        found: String,
    },
    NonGenericInstantiation,
    PrivateInstantiation,
    RequirementUnmet {
        ty: String,
        requirement: String,
        reason: String,
    },
    MemberAccessViolation,
    InvalidIndexAccess,
    InvalidDereference,
    UnsupportedGenericCall,
    IllegalVisibility,
    CannotInferenceType,
    VariableMustBeInitialized,
    NonConstantInitializer,
    ExpressionMustConditional,
    ReferenceCannotBeCalculated,
    InvalidAssignment,
    LoopControlOutsideLoop,
    ReturnOutsideFunction,
    ReturnValueRequired,
    UnexpectedReturnValue,
    ForBoundMustBeInteger,
    DuplicateDefaultBranch,
    ArgumentCountMismatch,
    TypeMismatched {
        expected: String,
        found: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CAbiError {
    GenericUnit,
    GenericFunction,
    UnitHasInterface,
    NonCAbiUnitParameter,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ErrorInfo {
    error: CompileError,
    span: Span,
}

impl ErrorInfo {
    pub fn new(error: CompileError, span: Span) -> Self {
        Self { error, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorHandle {
    pub file: String,
    pub errors: HashSet<ErrorInfo>,
}

impl ErrorHandle {
    pub(crate) fn first_error(&self) -> Option<CompileError> {
        self.errors.iter().next().map(|info| info.error.clone())
    }

    pub fn new(file: String) -> Self {
        Self {
            file,
            errors: HashSet::new(),
        }
    }

    pub fn get_context(
        &self,
        source: &str,
        pre_lines: usize,
        after_lines: usize,
        span: Range<usize>,
    ) -> String {
        render::context(source, pre_lines, after_lines, span)
    }

    pub fn render_error(&self, source: &str, span: Range<usize>, message: &str) -> String {
        render::diagnostic(&self.file, source, span, message)
    }

    pub fn render_warning(&self, source: &str, span: Range<usize>, message: &str) -> String {
        render::warning(&self.file, source, span, message)
    }

    pub fn render(&self, source: &str) -> String {
        let mut errors: Vec<_> = self.errors.iter().collect();
        errors.sort_by_key(|info| (info.span.start, info.span.end, info.error.to_string()));
        let diagnostics = errors
            .iter()
            .map(|info| self.render_error(source, info.span.into_range(), &info.error.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        Self::summarize(&diagnostics, errors.len())
    }

    pub fn summarize(diagnostics: &str, count: usize) -> String {
        format!(
            "{}\n{count} {} generated.",
            diagnostics.trim_end(),
            if count == 1 { "error" } else { "errors" }
        )
    }

    pub fn token_error(&self, token_error: TokenError, source: &str) {
        let message = token_error.message();
        eprintln!("{}", self.render_error(source, token_error.span, &message));
    }

    pub fn parse_error(&self, parse_error: &ParseError<'_>, source: &str) {
        let span = parse_error.span().into_range();
        eprintln!(
            "{}",
            self.render_error(
                source,
                span,
                &crate::parser::diagnostic::message(parse_error)
            )
        );
    }

    pub fn submit_error(&mut self, error: CompileError, span: Span) {
        let error_info = ErrorInfo::new(error, span);
        self.errors.insert(error_info);
    }
}
