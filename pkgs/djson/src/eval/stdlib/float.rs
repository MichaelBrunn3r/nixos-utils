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
pub fn create_map() -> Map<'static> {
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

pub const fn abs<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.abs())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn clamp<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [
            Value::Float(value),
            Value::Float(minimum),
            Value::Float(maximum),
        ] if minimum <= maximum => Ok(Value::Float(value.clamp(*minimum, *maximum))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ceil<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
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

pub fn floor<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
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

pub fn round<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
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

pub fn sqrt<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.sqrt())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ln<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.ln())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value), Value::Int(base)] => Ok(Value::Float(value.log(*base as f64))),
        [Value::Float(value), Value::Float(base)] => Ok(Value::Float(value.log(*base))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log2<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.log2())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log10<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Float(value.log10())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn max<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(left), Value::Float(right)] => Ok(Value::Float(left.max(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn min<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(left), Value::Float(right)] => Ok(Value::Float(left.min(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_finite<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_finite())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_infinite<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_infinite())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub const fn is_nan<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Float(value)] => Ok(Value::Bool(value.is_nan())),
        _ => Err(EvalError::TypeMismatch),
    }
}
