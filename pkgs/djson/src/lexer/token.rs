use miette::SourceSpan;

#[derive(Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: SourceSpan,
}

#[allow(clippy::derive_partial_eq_without_eq)]
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
    DotDot,

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
}
