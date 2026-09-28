#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    Add,
    Subtract,
    Negate,
    Multiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,

    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,

    Index,
    AddressOf,
    Dot,
    Dereference,
    Arrow,

    BitAnd,
    BitOr,
    BitXor,
    BitNot,

    LogicalAnd,
    LogicalOr,
    LogicalNot,
}

impl Operator {
    pub fn changes_binary_result_type(&self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::Less
                | Self::LessOrEqual
                | Self::Greater
                | Self::GreaterOrEqual
                | Self::LogicalAnd
                | Self::LogicalOr
                | Self::Index
        )
    }
}
