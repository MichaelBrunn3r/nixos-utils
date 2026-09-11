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
}

#[derive(Debug, PartialEq)]
pub struct Let<'input> {
    pub name: &'input str,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq)]
pub struct KV<'input> {
    pub key: &'input str,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'input> {
    Simple(&'input str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrefixOp {
    Positive,
    Negative,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InfixOp {
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
        op: PrefixOp,
        value: Box<Self>,
    },
    Binary {
        left: Box<Self>,
        op: InfixOp,
        right: Box<Self>,
    },
    Call {
        callee: Box<Self>,
        arguments: Vec<Self>,
    },
    If {
        condition: Box<Self>,
        then_branch: Box<Self>,
        else_branch: Box<Self>,
    },
}
