pub mod boolean;
pub mod float;
pub mod int;
pub mod list;
pub mod map;
pub mod math;
pub mod string;

use std::rc::Rc;

use crate::{
    eval::{EvalError, Value, scope::Scope},
    map,
};

/// Constructs the standard lexical prelude.
///
/// The prelude is installed as the root scope for normal evaluation and
/// always provides `import` for expression-based module loading.
#[must_use]
pub fn prelude() -> Rc<Scope<'static>> {
    Rc::new(Scope::from_values([("import", Value::Function(import))]))
}

#[must_use]
pub fn new() -> Rc<Scope<'static>> {
    prelude()
}

/// Imports a standard module by name.
///
/// # Errors
///
/// Returns [`EvalError::UnknownModule`] for an unknown module name and
/// [`EvalError::TypeMismatch`] when the argument is not a string.
pub fn import<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    match arguments {
        [Value::Str(name)] if name == "std" => Ok(Value::Map(std_map())),
        [Value::Str(name)] if name == "types" => Ok(Value::Map(types_map())),
        [Value::Str(name)] => Err(EvalError::UnknownModule(name.to_string())),
        _ => Err(EvalError::TypeMismatch),
    }
}

fn std_map() -> crate::eval::Map<'static> {
    map! {
        math: Value::Map(math::create_map()),
        types: Value::Map(types_map()),
    }
}

fn types_map() -> crate::eval::Map<'static> {
    map! {
        bool: Value::Map(boolean::create_map()),
        float: Value::Map(float::create_map()),
        int: Value::Map(int::create_map()),
        list: Value::Map(list::create_map()),
        map: Value::Map(map::create_map()),
        none: Value::Map(crate::eval::Map::new()),
        str: Value::Map(string::create_map()),
    }
}

pub(crate) fn type_member<'input>(value: &Value<'input>, name: &str) -> Option<Value<'input>> {
    let type_name = match value {
        Value::None => "none",
        Value::Bool(_) => "bool",
        Value::Float(_) => "float",
        Value::Int(_) => "int",
        Value::List(_) => "list",
        Value::Map(_) => "map",
        Value::Str(_) => "str",
        Value::Function(_) => return None,
    };

    let types = types_map();
    let Value::Map(module) = types.get(type_name)? else {
        return None;
    };
    module.get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::prelude;

    #[test]
    fn prelude_exposes_only_import() {
        let prelude = prelude();
        assert!(prelude.resolve("import").is_some());
        assert!(prelude.resolve("types").is_none());
        assert!(prelude.resolve("std").is_none());
    }
}
