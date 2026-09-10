#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map {
    map! {
        abs: Value::Function(abs),
        clamp: Value::Function(clamp),
        ln: Value::Function(ln),
        log: Value::Function(log),
        log10: Value::Function(log10),
        log2: Value::Function(log2),
        max: Value::Function(max),
        min: Value::Function(min),
        sqrt: Value::Function(sqrt),
    }
}

pub fn abs(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => value
            .checked_abs()
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn clamp(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value), Value::Int(minimum), Value::Int(maximum)] if minimum <= maximum => {
            Ok(Value::Int((*value).clamp(*minimum, *maximum)))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn sqrt(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).sqrt())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ln(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).ln())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value), Value::Int(base)] => {
            Ok(Value::Float((*value as f64).log(*base as f64)))
        }
        [Value::Int(value), Value::Float(base)] => Ok(Value::Float((*value as f64).log(*base))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log2(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).log2())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log10(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).log10())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn max(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(left), Value::Int(right)] => Ok(Value::Int((*left).max(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn min(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Int(left), Value::Int(right)] => Ok(Value::Int((*left).min(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        eval::{EvalError, Scope, Value, evaluate_ast, stdlib},
        parser::Parser,
        value,
    };

    fn evaluate(input: &str) -> Result<Value, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let mut scope = Scope::child(stdlib::new());
        evaluate_ast(&ast, &mut scope)
    }

    #[test]
    fn methods_can_be_invoked() {
        let cases = [
            ("(-2).abs()", value!(2)),
            ("12.clamp(0, 10)", value!(10)),
            ("1.ln()", value!(0.0)),
            ("8.log(2)", value!(3.0)),
            ("100.log10()", value!(2.0)),
            ("8.log2()", value!(3.0)),
            ("2.max(4)", value!(4)),
            ("2.min(4)", value!(2)),
            ("9.sqrt()", value!(3.0)),
        ];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), Ok(expected), "{expression}");
        }
    }

    #[test]
    fn expect_errors() {
        let cases = [("10.clamp(1)", Err(EvalError::TypeMismatch))];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), expected, "{expression}");
        }
    }
}
