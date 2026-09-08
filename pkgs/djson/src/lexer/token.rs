#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Id(&'a str),

    // Values
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'a str),

    // Operators
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    Eq,
    Equal,

    // Punctuation
    Sep,
    Colon,
    Dot,

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
}
