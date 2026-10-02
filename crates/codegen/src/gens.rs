use crate::gens::func::SymbolIr;
use std::collections::HashSet;

use crate::ast::arena::TypeIndex;
use crate::ast::function::{FuncSymbol, InterfaceSymbol};
use crate::ast::symbols;
use crate::ast::{Function, GlobalStatement, symbols::PackageSymbolTable, types::UnitType};

pub mod ast;
pub mod func;
pub mod value;
pub use value::LlvmValue;
pub mod error;
pub mod expr;
pub mod function;
pub mod globals;
pub mod instruction;
pub mod lowering;
mod memory;
pub mod variable_stack;

pub struct IrGenerator {
    used_symbols: HashSet<String>,
    symbol_order: Vec<String>,
    header: String,
    defines: String,
    declarations: Vec<(String, std::ops::Range<usize>)>,
    initializers: Vec<String>,
    bodys: String,
    errors: error::GenerationErrorHandle,
}

use crate::manifest::IR_RESERVE_SIZE;

impl IrGenerator {
    pub fn new(tiplet: String) -> IrGenerator {
        let mut header = format!("target triple =  \"{}\"\n", tiplet);

        let mut bodys = String::new();
        bodys.reserve(IR_RESERVE_SIZE);
        header.reserve(1024 * 10);

        IrGenerator {
            used_symbols: HashSet::new(),
            symbol_order: Vec::new(),
            header,
            defines: String::new(),
            declarations: Vec::new(),
            initializers: Vec::new(),
            bodys,
            errors: error::GenerationErrorHandle::default(),
        }
    }

    pub fn collect<T>(
        &mut self,
        task: impl Into<String>,
        work: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        let lengths = (self.header.len(), self.defines.len(), self.bodys.len());
        let symbol_count = self.symbol_order.len();
        let declaration_count = self.declarations.len();
        let initializer_count = self.initializers.len();
        let mut errors = std::mem::take(&mut self.errors);
        let output = errors.collect(task, || work(self));
        errors.append(std::mem::take(&mut self.errors));
        self.errors = errors;
        if output.is_none() {
            self.header.truncate(lengths.0);
            self.defines.truncate(lengths.1);
            self.bodys.truncate(lengths.2);
            self.declarations.truncate(declaration_count);
            self.initializers.truncate(initializer_count);
            while self.symbol_order.len() > symbol_count {
                if let Some(name) = self.symbol_order.pop() {
                    self.used_symbols.remove(&name);
                }
            }
        }
        output
    }

    pub fn error_handle(&self) -> &error::GenerationErrorHandle {
        &self.errors
    }

    fn remember_symbol(&mut self, name: String) -> bool {
        if !self.used_symbols.insert(name.clone()) {
            return false;
        }
        self.symbol_order.push(name);
        true
    }

    pub fn into_error_handle(self) -> error::GenerationErrorHandle {
        self.errors
    }

    pub fn finish(mut self) -> Result<String, error::GenerationErrors> {
        for (name, range) in &self.declarations {
            if !self.used_symbols.contains(&format!("definition::{name}")) {
                self.header.push_str(&self.defines[range.clone()]);
            }
        }
        self.header.push_str(&self.bodys);
        self.errors.finish(self.header)
    }

    pub fn add_unit(&mut self, unit: &UnitType, symbols: &PackageSymbolTable) {
        let unit_name = Self::unit_lowering(unit, symbols).to_string();
        if !self.remember_symbol(unit_name.clone()) {
            return;
        }

        let members = unit
            .members
            .iter()
            .map(|member| Self::type_lowering(member.member_type.clone(), symbols))
            .collect::<Vec<_>>();

        let struct_def = format!(
            "{} = type {{ {} }}\n",
            unit_name,
            members
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.header.push_str(&struct_def);
    }

    pub fn add_func_decl(
        &mut self,
        func: &Function,
        symbols: &PackageSymbolTable,
    ) -> func::LlvmFunc {
        match func {
            Function::Func(func) => {
                let symbol = symbols.get_function(func.symbol.clone(), false);
                self.add_function_decl(symbol, symbols)
            }
            Function::Interface(interface) => {
                let symbol = symbols.get_interface(interface.symbol.clone(), false);
                self.add_interface_decl(symbol, symbols)
            }
        }
    }

    pub fn generate(&mut self, package: &PackageSymbolTable, func: Function) {
        let file = match &func {
            Function::Func(f) => f.symbol.file_id(),
            Function::Interface(f) => f.symbol.file_id(),
        };
        self.generate_globals(file, package);
        // Include signature construction in the isolated task too.
        let task = match &func {
            Function::Func(f) => package.get_function(f.symbol, false).name.clone(),
            Function::Interface(f) => package.get_interface(f.symbol, false).name.clone(),
        };
        self.collect(task, |generator| {
            let abi = match &func {
                Function::Func(f) => package.get_function(f.symbol, false).llvm_func(package),
                Function::Interface(f) => package.get_interface(f.symbol, false).llvm_func(package),
            };
            let abi = abi.unwrap_or_else(|error| error.abort_generation());
            if !generator.remember_symbol(format!("definition::{}", abi.name())) {
                return;
            }
            let generated = function::FunctionGenerator::generate(&func, package);
            generator.add_resources(&generated.resources, &generated.intrinsics);
            let mut visited = HashSet::new();
            if let Some(ty) = abi.result().source {
                generator.ensure_type(ty, package, &mut visited);
            }
            for mapping in abi.receiver().into_iter().chain(abi.params()) {
                generator.ensure_type(mapping.source, package, &mut visited);
            }
            for variable in generated.variables.variables() {
                generator.ensure_type(variable.ty, package, &mut visited);
            }
            for call in &generated.calls {
                if let Some(ty) = call.result().source {
                    generator.ensure_type(ty, package, &mut visited);
                }
                for mapping in call.receiver().into_iter().chain(call.params()) {
                    generator.ensure_type(mapping.source, package, &mut visited);
                }
                generator.add_declaration(call);
            }
            generator.bodys.push_str(&generated.ir);
        });
    }

    fn ensure_type(
        &mut self,
        ty: TypeIndex,
        package: &PackageSymbolTable,
        visited: &mut HashSet<TypeIndex>,
    ) {
        if !visited.insert(ty) {
            return;
        }
        match package.get_type(ty).unqualified() {
            crate::ast::types::CompileType::Unit(unit) => {
                self.add_unit(unit, package);
                for member in &unit.members {
                    self.ensure_type(member.member_type, package, visited);
                }
            }
            crate::ast::types::CompileType::List(list) => {
                self.ensure_type(list.element_type, package, visited)
            }
            crate::ast::types::CompileType::Ref(reference) => {
                self.ensure_type(reference.base, package, visited)
            }
            _ => {}
        }
    }
}
