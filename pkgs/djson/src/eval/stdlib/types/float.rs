#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_errors_doc
)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map {
    map! {
        abs: Value::Function(abs),
        ceil: Value::Function(ceil),
        clamp: Value::Function(clamp),
        floor: Value::Function(floor),
        ln: Value::Function(ln),
        log: Value::Function(log),
        log10: Value::Function(log10),
        log2: Value::Function(log2),
        is_finite: Value::Function(is_finite),
        is_infinite: Value::Function(is_infinite),
        is_nan: Value::Function(is_nan),
        max: Value::Function(max),
        min: Value::Function(min),
        round: Value::Function(round),
        sqrt: Value::Function(sqrt),
    }
}

pub const fn abs(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.abs())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn clamp(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [
            Value::Float(value),
            Value::Float(minimum),
            Value::Float(maximum),
        ] if minimum <= maximum => Ok(Value::Float(value.clamp(*minimum, *maximum))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ceil(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => {
            let value = value.ceil();
            if !value.is_finite() || value < i64::MIN as f64 || value >= i64::MAX as f64 {
                return Err(EvalError::Overflow);
            }
            Ok(Value::Int(value as i64))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn floor(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => {
            let value = value.floor();
            if !value.is_finite() || value < i64::MIN as f64 || value >= i64::MAX as f64 {
                return Err(EvalError::Overflow);
            }
            Ok(Value::Int(value as i64))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn round(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => {
            let value = value.round();
            if !value.is_finite() || value < i64::MIN as f64 || value >= i64::MAX as f64 {
                return Err(EvalError::Overflow);
            }
            Ok(Value::Int(value as i64))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn sqrt(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.sqrt())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ln(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.ln())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value), Value::Int(base)] => Ok(Value::Float(value.log(*base as f64))),
        [Value::Float(value), Value::Float(base)] => Ok(Value::Float(value.log(*base))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log2(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.log2())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log10(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.log10())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn max(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(left), Value::Float(right)] => Ok(Value::Float(left.max(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn min(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(left), Value::Float(right)] => Ok(Value::Float(left.min(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_finite(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_finite())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_infinite(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_infinite())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_nan(arguments: &[Value]) -> Result<Value, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_nan())),
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
        let ast = Parser::new(input).parse_stmnts().expect("valid input");
        let mut scope = Scope::child(stdlib::new());
        evaluate_ast(&ast, &mut scope)
    }

    #[test]
    fn methods_can_be_invoked() {
        let import_stdlib = "let std = import('std')\n";
        let cases = [
            ("(-2.5).abs()", value!(2.5)),
            ("1.2.ceil()", value!(2)),
            ("12.5.clamp(0.0, 10.0)", value!(10.0)),
            ("1.8.floor()", value!(1)),
            ("1.0.ln()", value!(0.0)),
            ("8.0.log(2)", value!(3.0)),
            ("100.0.log10()", value!(2.0)),
            ("8.0.log2()", value!(3.0)),
            ("1.0.is_finite()", value!(true)),
            ("std.math.INFINITY.is_infinite()", value!(true)),
            ("std.math.NAN.is_nan()", value!(true)),
            ("2.5.max(4.0)", value!(4.0)),
            ("2.5.min(4.0)", value!(2.5)),
            ("1.5.round()", value!(2)),
            ("9.0.sqrt()", value!(3.0)),
        ];
        for (expression, expected) in cases {
            let expression = if expression.starts_with("std.") {
                format!("{import_stdlib}{expression}")
            } else {
                expression.to_owned()
            };
            assert_eq!(evaluate(&expression), Ok(expected), "{expression}");
        }
    }
}
