#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    MutOf,
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
    /// Larger values bind more tightly. Binary operators are left associative.
    pub fn precedence(&self) -> u8 {
        use Operator::*;
        match self {
            LogicalOr => 1,
            LogicalAnd => 2,
            BitOr => 3,
            BitXor => 4,
            BitAnd => 5,
            Equal | NotEqual => 6,
            Less | LessOrEqual | Greater | GreaterOrEqual => 7,
            ShiftLeft | ShiftRight => 8,
            Add | Subtract => 9,
            Multiply | Divide | Remainder => 10,
            _ => 11,
        }
    }

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

    pub fn is_logical(&self) -> bool {
        matches!(self, Self::LogicalAnd | Self::LogicalOr | Self::LogicalNot)
    }
}
