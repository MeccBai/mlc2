use super::CompileType;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DataType {
    Integer,
    Float,
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

    pub fn sizeof(&self) -> usize {
        self.bits / 8
    }

    pub fn name(&self) -> String {
        match self.data_type {
            DataType::Integer => {
                if self.signed {
                    format!("i{}", self.sizeof())
                } else {
                    format!("u{}", self.sizeof())
                }
            }
            DataType::Float => {
                format!("f{}", self.sizeof())
            }
        }
    }

    fn new(data_type: DataType, signed: bool, bits: usize) -> CompileType {
        CompileType::Base(Self {
            data_type,
            signed,
            bits,
        })
    }
    pub(super) fn base_types() -> [CompileType; 10] {
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
        ]
    }
}
