pub mod math;
pub mod types;

use std::rc::Rc;

use crate::{
    eval::{EvalError, Value, scope::Scope},
    map,
};

fn create_module() -> crate::eval::Map<'static> {
    map! {
        math: Value::Map(math::create_map()),
        types: Value::Map(types::create_module()),
    }
}

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

/// Imports a builtin module by name.
///
/// # Errors
///
/// Returns [`EvalError::UnknownModule`] for an unknown module name and
/// [`EvalError::TypeMismatch`] when the argument is not a string.
pub fn import<'input>(arguments: &[Value<'input>]) -> Result<Value<'input>, EvalError> {
    let [Value::Str(name)] = arguments else {
        return Err(EvalError::TypeMismatch);
    };

    match name.as_ref() {
        "std" => Ok(Value::Map(create_module())),
        "types" => Ok(Value::Map(types::create_module())),
        _ => Err(EvalError::UnknownModule(name.to_string())),
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

    let types = types::create_module();
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
