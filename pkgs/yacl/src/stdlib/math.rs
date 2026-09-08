#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::missing_errors_doc
)]

use crate::{
    eval::{EvalError, Value},
    scope::{Scope, Symbol},
};

pub const PI: Value<'static> = Value::Float(std::f64::consts::PI);
pub const INFINITY: Value<'static> = Value::Float(f64::INFINITY);
pub const NAN: Value<'static> = Value::Float(f64::NAN);

pub fn create_scope() -> Scope<'static> {
    Scope::from_symbols([
        ("PI", Symbol::Value(PI)),
        ("INFINITY", Symbol::Value(INFINITY)),
        ("NAN", Symbol::Value(NAN)),
        ("acos", Symbol::Function(acos)),
        ("asin", Symbol::Function(asin)),
        ("atan", Symbol::Function(atan)),
        ("atan2", Symbol::Function(atan2)),
        ("sin", Symbol::Function(sin)),
        ("cos", Symbol::Function(cos)),
        ("tan", Symbol::Function(tan)),
    ])
}

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

fn numeric_pair(arguments: &[Value<'_>]) -> Option<(f64, f64)> {
    match arguments {
        [left, right] => Some((as_float(left)?, as_float(right)?)),
        _ => None,
    }
}

const fn as_float(value: &Value<'_>) -> Option<f64> {
    match value {
        Value::Int(value) => Some(*value as f64),
        Value::Float(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{eval::evaluate_ast, parser::Parser};

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = crate::stdlib::new();
        evaluate_ast(&ast, &scope)
    }

    #[test]
    fn evaluates_math_builtins_through_the_pipeline() {
        let document = evaluate(
            "use std.math.*\nminimum = 4.min(2)\nmaximum = 2.5.max(4.0)\nlimited = 12.clamp(0, 10)\nlimited_float = 12.5.clamp(0.0, 10.0)\nnatural_log = 1.ln()\nbase_two_log = 8.log(2)\nbase_ten_log = 100.log(10)\ntangent = tan(0)\narcsine = asin(0)\narccosine = acos(1)\narctangent = atan(0)\nquadrant = atan2(1, 0)\nfinite = 1.0.is_finite()\ninfinite = 1.0.is_infinite()\nnan = 1.0.is_nan()\npi_value = PI\ninfinity_value = INFINITY\nnan_value = NAN",
        )
        .expect("math builtins should evaluate");

        let Value::Map(document) = document else {
            panic!("expected map")
        };
        assert_eq!(document.get("minimum"), Some(&Value::Int(2)));
        assert_eq!(document.get("maximum"), Some(&Value::Float(4.0)));
        assert_eq!(document.get("limited"), Some(&Value::Int(10)));
        assert_eq!(document.get("limited_float"), Some(&Value::Float(10.0)));
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
        assert_eq!(
            document.get("pi_value"),
            Some(&Value::Float(std::f64::consts::PI))
        );
        assert_eq!(
            document.get("infinity_value"),
            Some(&Value::Float(f64::INFINITY))
        );
        assert!(matches!(document.get("nan_value"), Some(Value::Float(value)) if value.is_nan()));
    }

    #[test]
    fn rejects_invalid_math_arguments_through_the_pipeline() {
        assert_eq!(
            evaluate("use std.math.*\nresult = cos(true)"),
            Err(EvalError::TypeMismatch)
        );
        assert_eq!(
            evaluate("use std.math.*\nresult = sin(1, 2)"),
            Err(EvalError::TypeMismatch)
        );
        assert_eq!(
            evaluate("result = 10.clamp(1)"),
            Err(EvalError::TypeMismatch)
        );
    }
}
