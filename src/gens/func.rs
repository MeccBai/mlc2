use super::{
    IrGenerator,
    instruction::{IrValue, LlvmType},
};
use crate::{
    ast::{
        arena::TypeIndex,
        attribute::FuncAttibute,
        function::{FuncSymbol, InterfaceSymbol},
        symbols::PackageSymbolTable,
        types::{CompileType, ValueType},
    },
    manifest::ABI_DIRECT_SIZE_LIMIT,
};
use std::{collections::HashSet, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterMode {
    Direct,
    ReadOnlyPointer,
    CopyPointer,
    ReturnPointer,
    Receiver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiParameter {
    /// Original value type; pointer modes retain it for call-site storage/copying.
    pub ty: LlvmType,
    pub mode: ParameterMode,
    pub align: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlvmFunc {
    name: String,
    result: ReturnMapping,
    receiver: Option<ParamMapping>,
    params: Vec<ParamMapping>,
    variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationError {
    GenericFunction,
    UnsupportedCAbiAggregate,
    InvalidVariadicMarker,
}
impl fmt::Display for DeclarationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IR declaration error: {self:?}")
    }
}
impl std::error::Error for DeclarationError {}

impl DeclarationError {
    pub(super) fn abort_generation(self) -> ! {
        let reason = self.to_string();
        match self {
            Self::UnsupportedCAbiAggregate => super::error::unsupported(&reason),
            _ => super::error::fail(&reason),
        }
    }
}

impl LlvmFunc {
    pub fn function(
        func: &FuncSymbol,
        symbols: &PackageSymbolTable,
    ) -> Result<Self, DeclarationError> {
        Self::new(
            &func.name,
            &func.generics,
            &func.attributes,
            &func.params,
            func.ret_type,
            None,
            symbols,
        )
    }

    pub fn interface(
        func: &InterfaceSymbol,
        symbols: &PackageSymbolTable,
    ) -> Result<Self, DeclarationError> {
        Self::new(
            &func.name,
            &func.generics,
            &func.attributes,
            &func.params,
            func.ret_type,
            func.has_self.then_some(func.owner),
            symbols,
        )
    }

    fn new(
        name: &str,
        generics: &[String],
        attributes: &HashSet<FuncAttibute>,
        params: &[(TypeIndex, String)],
        returns: Option<TypeIndex>,
        receiver: Option<TypeIndex>,
        symbols: &PackageSymbolTable,
    ) -> Result<Self, DeclarationError> {
        if !generics.is_empty() {
            return Err(DeclarationError::GenericFunction);
        }
        let c_abi = attributes.contains(&FuncAttibute::Cabi);
        let mut result = Self {
            name: if c_abi {
                name.rsplit("::").next().unwrap_or(name).into()
            } else {
                name.into()
            },
            result: ReturnMapping {
                source: returns,
                ty: LlvmType::Void,
                hidden: Vec::new(),
            },
            receiver: None,
            params: Vec::new(),
            variadic: false,
        };
        if let Some(ty) = returns {
            check_c_type(ty, c_abi, symbols)?;
            let lowered = IrGenerator::type_lowering(ty, symbols);
            if !c_abi && ty.size(symbols.arenas()) > ABI_DIRECT_SIZE_LIMIT {
                result.result.hidden.push(AbiParameter {
                    ty: lowered,
                    mode: ParameterMode::ReturnPointer,
                    align: ty.align(symbols.arenas()),
                });
            } else {
                result.result.ty = lowered;
            }
        }
        if let Some(owner) = receiver {
            result.receiver = Some(ParamMapping {
                source: owner,
                lowered: vec![AbiParameter {
                    ty: IrGenerator::type_lowering(owner, symbols),
                    mode: ParameterMode::Receiver,
                    align: owner.align(symbols.arenas()),
                }],
            });
        }
        for (i, (ty, name)) in params.iter().enumerate() {
            if name == "..." {
                if i + 1 != params.len() || !ty.is_empty() {
                    return Err(DeclarationError::InvalidVariadicMarker);
                }
                result.variadic = true;
                continue;
            }
            check_c_type(*ty, c_abi, symbols)?;
            let mode = if c_abi || ty.size(symbols.arenas()) <= ABI_DIRECT_SIZE_LIMIT {
                ParameterMode::Direct
            } else if symbols.get_type(*ty).value_type() == ValueType::Flex {
                ParameterMode::CopyPointer
            } else {
                ParameterMode::ReadOnlyPointer
            };
            result.params.push(ParamMapping {
                source: *ty,
                lowered: vec![AbiParameter {
                    ty: IrGenerator::type_lowering(*ty, symbols),
                    mode,
                    align: ty.align(symbols.arenas()),
                }],
            });
        }
        Ok(result)
    }
}

fn check_c_type(
    ty: TypeIndex,
    c_abi: bool,
    symbols: &PackageSymbolTable,
) -> Result<(), DeclarationError> {
    if c_abi
        && !matches!(
            symbols.get_type(ty).unqualified(),
            CompileType::Base(_) | CompileType::Ref(_)
        )
    {
        return Err(DeclarationError::UnsupportedCAbiAggregate);
    }
    Ok(())
}

impl fmt::Display for AbiParameter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mode {
            ParameterMode::Direct => write!(f, "{}", self.ty),
            ParameterMode::ReadOnlyPointer => write!(f, "ptr readonly align {}", self.align),
            ParameterMode::CopyPointer => write!(f, "ptr byval({}) align {}", self.ty, self.align),
            ParameterMode::ReturnPointer => write!(f, "ptr sret({}) align {}", self.ty, self.align),
            ParameterMode::Receiver => write!(f, "ptr align {}", self.align),
        }
    }
}

impl fmt::Display for LlvmFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "declare {} {}(",
            self.result.ty,
            IrValue::Global(self.name.clone())
        )?;
        for (i, param) in self.parameters().enumerate() {
            if i != 0 {
                f.write_str(", ")?;
            }
            write!(f, "{param}")?;
        }
        if self.variadic {
            if self.parameters().next().is_some() {
                f.write_str(", ")?;
            }
            f.write_str("...")?;
        }
        f.write_str(")")
    }
}

impl IrGenerator {
    pub fn add_function_decl(
        &mut self,
        func: &FuncSymbol,
        symbols: &PackageSymbolTable,
    ) -> LlvmFunc {
        let abi =
            LlvmFunc::function(func, symbols).unwrap_or_else(|error| error.abort_generation());
        self.add_declaration(&abi);
        abi
    }
    pub fn add_interface_decl(
        &mut self,
        func: &InterfaceSymbol,
        symbols: &PackageSymbolTable,
    ) -> LlvmFunc {
        let abi =
            LlvmFunc::interface(func, symbols).unwrap_or_else(|error| error.abort_generation());
        self.add_declaration(&abi);
        abi
    }
    pub(super) fn add_declaration(&mut self, abi: &LlvmFunc) {
        use std::fmt::Write;
        if self.remember_symbol(abi.name.clone()) {
            let start = self.defines.len();
            writeln!(&mut self.defines, "{abi}").expect("String write");
            self.declarations
                .push((abi.name.clone(), start..self.defines.len()));
        }
    }
}

mod call;
pub use call::CallError;
#[cfg(test)]
mod tests;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamMapping {
    pub source: TypeIndex,
    /// Source parameter maps to zero, one or more LLVM parameters.
    pub lowered: Vec<AbiParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnMapping {
    pub source: Option<TypeIndex>,
    pub ty: LlvmType,
    pub hidden: Vec<AbiParameter>,
}

impl LlvmFunc {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn result(&self) -> &ReturnMapping {
        &self.result
    }
    pub fn receiver(&self) -> Option<&ParamMapping> {
        self.receiver.as_ref()
    }
    pub fn params(&self) -> &[ParamMapping] {
        &self.params
    }
    pub fn is_variadic(&self) -> bool {
        self.variadic
    }

    pub fn parameters(&self) -> impl Iterator<Item = &AbiParameter> {
        self.result
            .hidden
            .iter()
            .chain(self.receiver.iter().flat_map(|p| p.lowered.iter()))
            .chain(self.params.iter().flat_map(|p| p.lowered.iter()))
    }
}

impl FuncSymbol {
    pub fn llvm_func(&self, symbols: &PackageSymbolTable) -> Result<LlvmFunc, DeclarationError> {
        LlvmFunc::function(self, symbols)
    }
}
impl InterfaceSymbol {
    pub fn llvm_func(&self, symbols: &PackageSymbolTable) -> Result<LlvmFunc, DeclarationError> {
        LlvmFunc::interface(self, symbols)
    }
}
