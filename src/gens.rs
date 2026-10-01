use std::collections::HashSet;

use crate::ast::{Function, GlobalStatement, symbols::PackageSymbolTable, types::UnitType};

pub struct IrGenerator {
    used_symbols: HashSet<String>,
    header: String,
    defines: String,
    bodys: String,
}

const RESERVE_SIZE: usize = 2 * 1000 * 1000;

impl IrGenerator {
    pub fn new(tiplet: String) -> IrGenerator {
        let triplet_tip = format!("target triple =  \"{}\"\n", tiplet);

        let mut bodys = String::new();

        bodys.reserve(RESERVE_SIZE);

        IrGenerator {
            used_symbols: HashSet::new(),
            header: triplet_tip,
            defines: String::new(),
            bodys: bodys,
        }
    }

    pub fn add_unit(&mut self, unit: &UnitType) {

        

    }

    pub fn generate(&mut self, package: &PackageSymbolTable, func: Function) {
        match func {
            Function::Func(func) => {
                let symbol = func.symbol;
                let body = func.body;
            }
            Function::Interface(interface) => {
                let symbol = interface.symbol;
                let body = interface.body;
            }
        }
    }
}
