#[derive(Debug, PartialEq)]
pub struct AST<'input> {
    pub pairs: Vec<KV<'input>>,
}

#[derive(Debug, PartialEq)]
pub struct KV<'input> {
    pub key: &'input str,
    pub expr: Expr<'input>,
}

#[derive(Debug, PartialEq)]
pub enum UnaryOp {
    Positive,
    Negative,
}

#[derive(Debug, PartialEq)]
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
    Id(&'input str),
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
        name: &'input str,
        arguments: Vec<Self>,
    },
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub fn int(value: i64) -> Expr<'static> {
        Expr::Int(value)
    }

    pub fn op_unary<'input>(op: UnaryOp, value: Expr<'input>) -> Expr<'input> {
        Expr::Unary {
            op,
            value: Box::new(value),
        }
    }

    pub fn op_binary<'input>(
        left: Expr<'input>,
        op: BinaryOp,
        right: Expr<'input>,
    ) -> Expr<'input> {
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    pub fn add<'input>(left: Expr<'input>, right: Expr<'input>) -> Expr<'input> {
        op_binary(left, BinaryOp::Add, right)
    }

    pub fn sub<'input>(left: Expr<'input>, right: Expr<'input>) -> Expr<'input> {
        op_binary(left, BinaryOp::Sub, right)
    }

    pub fn mul<'input>(left: Expr<'input>, right: Expr<'input>) -> Expr<'input> {
        op_binary(left, BinaryOp::Mul, right)
    }

    pub fn div<'input>(left: Expr<'input>, right: Expr<'input>) -> Expr<'input> {
        op_binary(left, BinaryOp::Div, right)
    }

    pub fn exp<'input>(left: Expr<'input>, right: Expr<'input>) -> Expr<'input> {
        op_binary(left, BinaryOp::Exp, right)
    }

    pub fn op_pos<'input>(value: Expr<'input>) -> Expr<'input> {
        op_unary(UnaryOp::Positive, value)
    }

    pub fn op_neg<'input>(value: Expr<'input>) -> Expr<'input> {
        op_unary(UnaryOp::Negative, value)
    }

    pub fn call<'input>(name: &'input str, arguments: Vec<Expr<'input>>) -> Expr<'input> {
        Expr::Call { name, arguments }
    }

    pub fn assert_pairs(document: AST<'_>, expected: &[(&str, Expr<'_>)]) {
        assert_eq!(document.pairs.len(), expected.len());

        for (pair, (expected_key, expected_value)) in document.pairs.iter().zip(expected) {
            assert_eq!(pair.key, *expected_key);
            assert_eq!(&pair.expr, expected_value);
        }
    }
}
