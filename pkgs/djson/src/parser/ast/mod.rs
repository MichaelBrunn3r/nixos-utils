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
    pub pattern: Pattern<'input>,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq)]
pub enum Pattern<'input> {
    Name(&'input str),
    Map(Vec<MapPattern<'input>>),
    List {
        patterns: Vec<Self>,
        rest: Option<&'input str>,
    },
}

#[derive(Debug, PartialEq)]
pub struct MapPattern<'input> {
    pub key: &'input str,
    pub pattern: Pattern<'input>,
    pub default: Option<Expr<'input>>,
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
        then: Box<Self>,
        r#else: Box<Self>,
    },
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
