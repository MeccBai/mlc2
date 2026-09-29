use crate::error::ice::ice;

use super::CompileType;
use crate::ast::symbol_name::SymbolName;
use std::collections::HashSet;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DataType {
    Integer,
    Float,
    Boolean,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BaseType {
    data_type: DataType,
    signed: bool,
    bits: usize,
}

impl BaseType {
    pub fn data_type(&self) -> DataType {
        self.data_type.clone()
    }

    pub fn bits(&self) -> usize {
        self.bits
    }

    pub fn signed(&self) -> bool {
        self.signed
    }

    pub fn size(&self) -> usize {
        match self.data_type {
            DataType::Integer => self.bits / 8,
            DataType::Float => self.bits / 8,
            DataType::Boolean => 1,
        }
    }

    pub fn align(&self) -> usize {
        self.size()
    }

    pub fn name(&self) -> String {
        SymbolName::base_type(self.data_type.clone(), self.signed, self.bits)
    }

    pub fn format(&self) -> String {
        self.name()
    }

    pub fn dump(&self) -> String {
        self.name()
    }

    fn new(data_type: DataType, signed: bool, bits: usize) -> CompileType {
        CompileType::Base(Self {
            data_type,
            signed,
            bits,
        })
    }

    pub fn base_types() -> Vec<(usize, CompileType)> {
        [
            Self::new(DataType::Integer, true, 8),
            Self::new(DataType::Integer, true, 16),
            Self::new(DataType::Integer, true, 32),
            Self::new(DataType::Integer, true, 64),
            Self::new(DataType::Integer, false, 8),
            Self::new(DataType::Integer, false, 16),
            Self::new(DataType::Integer, false, 32),
            Self::new(DataType::Integer, false, 64),
            Self::new(DataType::Float, true, 32),
            Self::new(DataType::Float, true, 64),
            Self::new(DataType::Boolean, false, 1),
        ]
        .into_iter()
        .zip(0..100)
        .map(|(ty, index)| (index, ty))
        .collect::<Vec<_>>()
    }

    pub fn to_index(data_type: DataType, bits: usize, signed: bool) -> usize {
        match data_type {
            DataType::Integer => {
                if signed {
                    match bits {
                        8 => 0,
                        16 => 1,
                        32 => 2,
                        64 => 3,
                        _ => ice("Invalid integer size"),
                    }
                } else {
                    match bits {
                        8 => 4,
                        16 => 5,
                        32 => 6,
                        64 => 7,
                        _ => ice("Invalid unsigned integer size"),
                    }
                }
            }
            DataType::Float => match bits {
                32 => 8,
                64 => 9,
                _ => ice("Invalid float size"),
            },
            DataType::Boolean => 10,
        }
    }

    pub fn type_check(&self, tolerance: bool, other: &BaseType) -> bool {
        match (self.data_type.clone(), other.data_type.clone()) {
            (DataType::Integer, DataType::Integer) => {
                if tolerance {
                    self.bits <= other.bits
                } 
                else {
                    self.bits == other.bits && self.signed == other.signed
                }
            }
            (DataType::Float, DataType::Float) => {
                if tolerance {
                    self.bits <= other.bits
                } 
                else {
                    self.bits == other.bits
                }
            }
            (DataType::Boolean, DataType::Boolean) => true,
            _ => false,
        }
    }
}
