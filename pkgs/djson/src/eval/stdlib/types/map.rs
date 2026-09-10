#![allow(clippy::missing_errors_doc)]

use crate::{
    eval::{EvalError, Map, Value},
    map,
};

#[must_use]
pub fn create_map() -> Map<'static> {
    map! {
        equals: Value::Function(equals),
        is_empty: Value::Function(is_empty),
        keys: Value::Function(keys),
        len: Value::Function(len),
        values: Value::Function(values),
    }
}

pub fn equals<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Map(left), Value::Map(right)] => Ok(Value::Bool(left == right)),
        _ => Err(EvalError::TypeMismatch),
    }
}

pub fn is_empty<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Map(map)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    Ok(Value::Bool(map.is_empty()))
}

pub fn keys<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Map(values)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    Ok(Value::List(
        values.keys().map(|key| Value::Str(key.into())).collect(),
    ))
}

pub fn values<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Map(map)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    Ok(Value::List(map.values().cloned().collect()))
}

pub fn len<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Map(map)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    let Ok(length) = i64::try_from(map.len()) else {
        return Err(EvalError::Overflow);
    };
    Ok(Value::Int(length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        eval::{evaluate_ast, stdlib},
        parser::Parser,
        value,
    };

    fn evaluate(input: &str) -> Result<Value<'_>, EvalError> {
        let ast = Parser::new(input).parse().expect("valid input");
        let scope = stdlib::new();
        evaluate_ast(&ast, &scope)
    }

    #[test]
    fn methods_can_be_invoked() {
        let cases = [
            (
                "{b: 2, a: 1, c: 3}.equals({a: 1, b: 2, c: 3})",
                value!(true),
            ),
            ("{}.is_empty()", value!(true)),
            ("{b: 2, a: 1, c: 3}.keys()", value!(["a", "b", "c"])),
            ("{b: 2, a: 1, c: 3}.len()", value!(3)),
            ("{b: 2, a: 1, c: 3}.values()", value!([1, 2, 3])),
        ];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), Ok(expected), "{expression}");
        }
    }

    #[test]
    fn expect_errors() {
        let cases = [("{}.keys(1)", Err(EvalError::TypeMismatch))];
        for (expression, expected) in cases {
            assert_eq!(evaluate(expression), expected, "{expression}");
        }
    }
}
