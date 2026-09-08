use std::{borrow::Cow, collections::BTreeMap};

use crate::eval::EvalError;

pub type Document<'input> = Map<'input>;
pub type Map<'input> = BTreeMap<&'input str, Value<'input>>;
pub type BuiltinFunction = for<'input> fn(&[Value<'input>]) -> Result<Value<'input>, EvalError>;

#[derive(Debug, Clone)]
pub enum Value<'input> {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Cow<'input, str>),
    List(Vec<Self>),
    Map(Map<'input>),
    Function(BuiltinFunction),
}

impl PartialEq for Value<'_> {
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
