pub mod map;

use crate::eval::{EvalError, document::map::Map};

pub type Document = Map;
pub type BuiltinFunction = fn(&[Value]) -> Result<Value, EvalError>;

#[derive(Debug, Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Self>),
    Map(Map),
    Function(BuiltinFunction),
}

#[macro_export]
macro_rules! value {
    (none) => {
        $crate::eval::Value::None
    };
    (null) => {
        $crate::eval::Value::None
    };
    (nil) => {
        $crate::eval::Value::None
    };
    ([$($value:tt),* $(,)?]) => {
        $crate::eval::Value::List(vec![$($crate::value!($value)),*])
    };
    ({$($key:tt : $value:tt),* $(,)?}) => {
        $crate::eval::Value::Map($crate::map! {
            $($key: $crate::value!($value)),*
        })
    };
    ($value:expr) => {
        $crate::eval::Value::from($value)
    };
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Str(left), Self::Str(right)) => left == right,
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Map(left), Self::Map(right)) => left == right,
            _ => false,
        }
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Int(i64::from(value))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::map;

    #[test]
    fn value_macro_constructs_scalars_and_lists() {
        assert_eq!(value!(42), Value::Int(42));
        assert_eq!(value!(true), Value::Bool(true));
        assert_eq!(value!("hello"), Value::from("hello"));
        assert_eq!(
            value!([1, "two"]),
            Value::List(vec![Value::Int(1), Value::from("two")])
        );
    }

    #[test]
    fn value_macro_constructs_maps_and_none() {
        assert_eq!(value!(none), Value::None);
        assert_eq!(value!(null), Value::None);
        assert_eq!(value!(nil), Value::None);
        assert_eq!(
            value!({ answer: 42, enabled: true }),
            Value::Map(map! { answer: 42, enabled: true })
        );
    }
}
