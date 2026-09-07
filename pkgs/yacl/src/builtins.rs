#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_errors_doc
)]

use crate::eval::{EvalError, Value};

//region Constants
pub const PI: Value<'static> = Value::Float(std::f64::consts::PI);
//endregion Constants

//region Trigonometry
pub fn sin<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::sin)
}

pub fn cos<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::cos)
}

pub fn tan<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::tan)
}

pub fn asin<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::asin)
}

pub fn acos<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::acos)
}

pub fn atan<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::atan)
}

pub fn atan2<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match numeric_pair(arguments) {
        Some((left, right)) => Ok(Value::Float(left.atan2(right))),
        None => Err(EvalError::TypeMismatch),
    }
}
//endregion

//region Numeric Functions
pub fn abs<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => value
            .checked_abs()
            .map(Value::Int)
            .ok_or(EvalError::Overflow),
        [Value::Float(value)] => Ok(Value::Float(value.abs())),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn min<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    numeric_extremum(arguments, f64::min, i64::min)
}

pub fn max<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    numeric_extremum(arguments, f64::max, i64::max)
}

pub fn clamp<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value), Value::Int(minimum), Value::Int(maximum)] if minimum <= maximum => {
            Ok(Value::Int((*value).clamp(*minimum, *maximum)))
        }
        [value, minimum, maximum] => {
            let (value, minimum, maximum) =
                numeric_values(&[value.clone(), minimum.clone(), maximum.clone()])?;
            clamp_float(value, minimum, maximum)
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn sqrt<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::sqrt)
}

pub fn ln<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::ln)
}

pub fn log<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match numeric_pair(arguments) {
        Some((value, base)) => Ok(Value::Float(value.log(base))),
        None => Err(EvalError::TypeMismatch),
    }
}

pub fn log2<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::log2)
}

pub fn log10<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_float(arguments, f64::log10)
}

pub fn is_nan<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_predicate(arguments, f64::is_nan)
}

pub fn is_infinite<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_predicate(arguments, f64::is_infinite)
}

pub fn is_finite<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    unary_predicate(arguments, f64::is_finite)
}

fn unary_float<'input>(
    arguments: &[Value<'input>],
    operation: fn(f64) -> f64,
) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Float(operation(*value as f64))),
        [Value::Float(value)] => Ok(Value::Float(operation(*value))),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn unary_predicate<'input>(
    arguments: &[Value<'input>],
    predicate: fn(f64) -> bool,
) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Bool(predicate(*value as f64))),
        [Value::Float(value)] => Ok(Value::Bool(predicate(*value))),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn numeric_pair(arguments: &[Value<'_>]) -> Option<(f64, f64)> {
    match arguments {
        [left, right] => Some((as_float(left)?, as_float(right)?)),
        _ => None,
    }
}

fn numeric_values(arguments: &[Value<'_>]) -> Result<(f64, f64, f64), EvalError> {
    match arguments {
        [value, minimum, maximum] => Ok((
            as_float(value).ok_or(EvalError::TypeMismatch)?,
            as_float(minimum).ok_or(EvalError::TypeMismatch)?,
            as_float(maximum).ok_or(EvalError::TypeMismatch)?,
        )),
        _ => Err(EvalError::TypeMismatch),
    }
}

const fn as_float(value: &Value<'_>) -> Option<f64> {
    match value {
        Value::Int(value) => Some(*value as f64),
        Value::Float(value) => Some(*value),
        _ => None,
    }
}

fn numeric_extremum<'input>(
    arguments: &[Value<'input>],
    float_operation: fn(f64, f64) -> f64,
    int_operation: fn(i64, i64) -> i64,
) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(left), Value::Int(right)] => Ok(Value::Int(int_operation(*left, *right))),
        [left, right] => {
            let (left, right) =
                numeric_pair(&[left.clone(), right.clone()]).ok_or(EvalError::TypeMismatch)?;
            Ok(Value::Float(float_operation(left, right)))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}

fn clamp_float<'input>(value: f64, minimum: f64, maximum: f64) -> Result<Value<'input>, EvalError> {
    if minimum > maximum {
        return Err(EvalError::TypeMismatch);
    }
    Ok(Value::Float(value.clamp(minimum, maximum)))
}
//endregion Numeric Functions

//region Rounding
pub fn floor<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    round_value(arguments, f64::floor)
}

pub fn ceil<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    round_value(arguments, f64::ceil)
}

pub fn round<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    round_value(arguments, f64::round)
}

fn round_value<'input>(
    arguments: &[Value<'input>],
    operation: fn(f64) -> f64,
) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Int(value)] => Ok(Value::Int(*value)),
        [Value::Float(value)] => {
            let value = operation(*value);
            if !value.is_finite() || value < i64::MIN as f64 || value >= i64::MAX as f64 {
                return Err(EvalError::Overflow);
            }
            Ok(Value::Int(value as i64))
        }
        _ => Err(EvalError::TypeMismatch),
    }
}
//endregion Rounding

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::evaluate_ast, parser::Parser, scope::Scope};

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = Scope::root();
        evaluate_ast(&ast, &scope)
    }

    #[test]
    fn evaluates_builtins_through_the_pipeline() {
        let document = evaluate(
            "use std.*\nconstant = pi\nsine = sin(pi / 2)\ncosine = cos(pi)\ndown = floor(3.7)\nup = ceil(-3.7)\nnearest = round(sin(pi / 2))",
        )
        .expect("builtins should evaluate");

        let Value::Map(document) = document else {
            panic!("expected map")
        };
        assert_eq!(document.get("constant"), Some(&PI));
        assert_eq!(document.get("sine"), Some(&Value::Float(1.0)));
        assert_eq!(document.get("cosine"), Some(&Value::Float(-1.0)));
        assert_eq!(document.get("down"), Some(&Value::Int(3)));
        assert_eq!(document.get("up"), Some(&Value::Int(-3)));
        assert_eq!(document.get("nearest"), Some(&Value::Int(1)));
    }

    #[test]
    fn evaluates_math_builtins_through_the_pipeline() {
        let document = evaluate(
            "use std.*\nabsolute = abs(-3)\nminimum = min(4, 2.5)\nmaximum = max(4, 2.5)\nlimited = clamp(12, 0, 10)\nroot = sqrt(9)\nnatural_log = ln(1)\nbase_two_log = log(8, 2)\nbase_ten_log = log(100, 10)\ntangent = tan(0)\narcsine = asin(0)\narccosine = acos(1)\narctangent = atan(0)\nquadrant = atan2(1, 0)\nfinite = is_finite(1)\ninfinite = is_infinite(1)\nnan = is_nan(1)",
        )
        .expect("math builtins should evaluate");

        let Value::Map(document) = document else {
            panic!("expected map")
        };
        assert_eq!(document.get("absolute"), Some(&Value::Int(3)));
        assert_eq!(document.get("minimum"), Some(&Value::Float(2.5)));
        assert_eq!(document.get("maximum"), Some(&Value::Float(4.0)));
        assert_eq!(document.get("limited"), Some(&Value::Int(10)));
        assert_eq!(document.get("root"), Some(&Value::Float(3.0)));
        assert_eq!(document.get("natural_log"), Some(&Value::Float(0.0)));
        assert_eq!(document.get("base_two_log"), Some(&Value::Float(3.0)));
        assert_eq!(document.get("base_ten_log"), Some(&Value::Float(2.0)));
        assert_eq!(document.get("tangent"), Some(&Value::Float(0.0)));
        assert_eq!(document.get("arcsine"), Some(&Value::Float(0.0)));
        assert_eq!(document.get("arccosine"), Some(&Value::Float(0.0)));
        assert_eq!(document.get("arctangent"), Some(&Value::Float(0.0)));
        assert_eq!(
            document.get("quadrant"),
            Some(&Value::Float(std::f64::consts::FRAC_PI_2))
        );
        assert_eq!(document.get("finite"), Some(&Value::Bool(true)));
        assert_eq!(document.get("infinite"), Some(&Value::Bool(false)));
        assert_eq!(document.get("nan"), Some(&Value::Bool(false)));
    }

    #[test]
    fn rejects_invalid_builtin_arguments_through_the_pipeline() {
        assert_eq!(
            evaluate("use std.*\nresult = cos(true)"),
            Err(EvalError::TypeMismatch)
        );
        assert_eq!(
            evaluate("use std.*\nresult = round(1, 2)"),
            Err(EvalError::TypeMismatch)
        );
        assert_eq!(
            evaluate("use std.*\nresult = log(10)"),
            Err(EvalError::TypeMismatch)
        );
    }
}
