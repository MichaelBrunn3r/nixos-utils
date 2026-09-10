#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map<'static> {
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

pub fn abs<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => value
            .checked_abs()
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn clamp<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value), Value::Int(minimum), Value::Int(maximum)] if minimum <= maximum => {
            Ok(Value::Int((*value).clamp(*minimum, *maximum)))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn sqrt<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).sqrt())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn ln<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).ln())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value), Value::Int(base)] => {
            Ok(Value::Float((*value as f64).log(*base as f64)))
        }
        [Value::Int(value), Value::Float(base)] => Ok(Value::Float((*value as f64).log(*base))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log2<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).log2())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn log10<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float((*value as f64).log10())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn max<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(left), Value::Int(right)] => Ok(Value::Int((*left).max(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn min<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(left), Value::Int(right)] => Ok(Value::Int((*left).min(*right))),
        _ => Err(EvalError::TypeMismatch),
    }
}
