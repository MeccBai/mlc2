use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    SingleLineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    MultiLineComment,

    #[regex(r"[0-9]*\.[0-9]+([eE][-+]?[0-9]+)?|[0-9]+\.[0-9]*([eE][-+]?[0-9]+)?|[0-9]+[eE][-+]?[0-9]+")]
    FloatLiteral,

    #[regex("[0-9]+")]
    IntLiteral,

    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,

    #[token("[[")]
    AttributeStart,
    #[token("]]")]
    AttributeEnd,
    #[token("|>")]
    Pipe,
    #[token("::")]
    ColonColon,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token(">")]
    RAngle,
    #[token("<")]
    LAngle,
    #[token(">=")]
    GreaterOrEqual,
    #[token("<=")]
    LessOrEqual,
    #[token("&&")]
    LogicalAnd,
    #[token("||")]
    LogicalOr,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("..")]
    Until,
    #[token("=>")]
    FatArrow,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(";")]
    Semicolon,
    #[token("=")]
    Assign,
    #[token("@")]
    AddressOf,
    #[token("$")]
    Dereference,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,

    #[token("->")]
    Arrow,
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    #[token("!")]
    LogicalNot,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multi,
    #[token("/")]
    Div,
    #[token("%")]
    Mod,

    #[token("&")]
    BitAnd,
    #[token("|")]
    BitOr,
    #[token("^")]
    BitXor,
    #[token("~")]
    BitNot,

    #[token("func")]
    Function,
    #[token("var")]
    Variable,
    #[token("const")]
    Constant,
    #[token("unit")]
    Unit,
    #[token("using")]
    Using,
    #[token("generic")]
    Generic,
    #[token("enum")]
    Enum,

    #[token("import")]
    Import,
    #[token("export")]
    Export,
    #[token("pub")]
    Public,
    #[token("global")]
    Global,

    #[token("in")]
    In,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("break")]
    Break,
    #[token("continue")]
    Continue,
    #[token("match")]
    Match,
    #[token("return")]
    Return,
    #[token("null")]
    Null,

    #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident,
}

impl Token {
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Token::Assign
                | Token::AddressOf
                | Token::Dereference
                | Token::Arrow
                | Token::Dot
                | Token::Comma
                | Token::Colon
                | Token::ColonColon
                | Token::Plus
                | Token::Minus
                | Token::Multi
                | Token::Div
                | Token::Mod
                | Token::BitAnd
                | Token::BitOr
                | Token::BitXor
                | Token::BitNot
                | Token::Shl
                | Token::Shr
                | Token::Equal
                | Token::NotEqual
                | Token::RAngle
                | Token::LAngle
                | Token::GreaterOrEqual
                | Token::LessOrEqual
                | Token::LogicalAnd
                | Token::LogicalOr
                | Token::LogicalNot
        )
    }

    pub fn is_sub_scope_start(&self) -> bool {
        matches!(self, Token::If | Token::Else | Token::While | Token::For | Token::LBrace)
    }
}
