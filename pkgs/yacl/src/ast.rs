#[derive(Debug, PartialEq)]
pub struct AST<'input> {
    pub statements: Vec<Statement<'input>>,
}

#[derive(Debug, PartialEq)]
pub enum Statement<'input> {
    Expr(Expr<'input>),
    KV(KV<'input>),
    Use(Use<'input>),
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
}

#[derive(Debug, PartialEq)]
pub enum Expr<'input> {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'input str),
    Id(Identifier<'input>),
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
        path: Vec<&'input str>,
        arguments: Vec<Self>,
    },
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    #[must_use]
    pub fn int(value: i64) -> Expr<'static> {
        Expr::Int(value)
    }

    #[must_use]
    pub fn op_unary(op: UnaryOp, value: Expr<'_>) -> Expr<'_> {
        Expr::Unary {
            op,
            value: Box::new(value),
        }
    }

    #[must_use]
    pub fn op_binary<'a>(left: Expr<'a>, op: BinaryOp, right: Expr<'a>) -> Expr<'a> {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    #[must_use]
    pub fn add<'a>(left: Expr<'a>, right: Expr<'a>) -> Expr<'a> {
        op_binary(left, BinaryOp::Add, right)
    }

    #[must_use]
    pub fn sub<'a>(left: Expr<'a>, right: Expr<'a>) -> Expr<'a> {
        op_binary(left, BinaryOp::Sub, right)
    }

    #[must_use]
    pub fn mul<'a>(left: Expr<'a>, right: Expr<'a>) -> Expr<'a> {
        op_binary(left, BinaryOp::Mul, right)
    }

    #[must_use]
    pub fn div<'a>(left: Expr<'a>, right: Expr<'a>) -> Expr<'a> {
        op_binary(left, BinaryOp::Div, right)
    }

    #[must_use]
    pub fn exp<'a>(left: Expr<'a>, right: Expr<'a>) -> Expr<'a> {
        op_binary(left, BinaryOp::Exp, right)
    }

    #[must_use]
    pub fn op_pos(value: Expr<'_>) -> Expr<'_> {
        op_unary(UnaryOp::Positive, value)
    }

    #[must_use]
    pub fn op_neg(value: Expr<'_>) -> Expr<'_> {
        op_unary(UnaryOp::Negative, value)
    }

    #[must_use]
    pub fn call<'a>(name: &'a str, arguments: Vec<Expr<'a>>) -> Expr<'a> {
        Expr::Call {
            path: vec![name],
            arguments,
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn assert_pairs(document: &AST<'_>, expected: &[(&str, Expr<'_>)]) {
        assert_eq!(document.statements.len(), expected.len());

        for (statement, (expected_key, expected_value)) in document.statements.iter().zip(expected)
        {
            let Statement::KV(pair) = statement else {
                panic!("expected an entry statement")
            };
            assert_eq!(pair.key, *expected_key);
            assert_eq!(&pair.expr, expected_value);
        }
    }
}
