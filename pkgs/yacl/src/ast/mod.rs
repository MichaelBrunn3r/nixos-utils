#[cfg(any(test, feature = "pretty_ast"))]
pub mod pretty;

#[derive(Debug, PartialEq)]
pub struct AST<'input> {
    pub statements: Vec<Statement<'input>>,
}

#[derive(Debug, PartialEq)]
pub enum Statement<'input> {
    Expr(Expr<'input>),
    KV(KV<'input>),
    Let(Let<'input>),
    Use(Use<'input>),
}

#[derive(Debug, PartialEq)]
pub struct Let<'input> {
    pub name: &'input str,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Use<'input> {
    pub path: Vec<&'input str>,
    pub wildcard: bool,
}

#[derive(Debug, PartialEq)]
pub struct KV<'input> {
    pub key: &'input str,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'input> {
    Simple(&'input str),
    Qualified(Vec<&'input str>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Positive,
    Negative,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Exp,
    Equal,
}

#[derive(Debug, PartialEq)]
pub enum Expr<'input> {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'input str),
    List(Vec<Self>),
    Map(Vec<KV<'input>>),
    Id(Identifier<'input>),
    Access {
        object: Box<Self>,
        name: &'input str,
    },
    Unary {
        op: UnaryOp,
        value: Box<Self>,
    },
    Binary {
        left: Box<Self>,
        op: BinaryOp,
        right: Box<Self>,
    },
    Call {
        callee: Box<Self>,
        arguments: Vec<Self>,
    },
}
