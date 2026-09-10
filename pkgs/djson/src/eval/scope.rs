use std::{collections::BTreeMap, rc::Rc};

use crate::eval::{EvalError, Value};

#[derive(Clone, Debug)]
pub struct Scope {
    values: BTreeMap<String, Value>,
    parent: Option<Rc<Self>>,
}

impl Scope {
    #[must_use]
    pub fn from_values<K>(values: impl IntoIterator<Item = (K, Value)>) -> Self
    where
        K: Into<String>,
    {
        Self {
            values: values
                .into_iter()
                .map(|(name, value)| (name.into(), value))
                .collect(),
            parent: None,
        }
    }

    #[must_use]
    pub const fn child(parent: Rc<Self>) -> Self {
        Self {
            values: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    /// Binds a value in this scope without allowing same-scope redeclaration.
    ///
    /// # Errors
    ///
    /// Returns [`EvalError::SymbolConflict`] when the name is already bound in
    /// this scope.
    pub fn bind_value(&mut self, name: &str, value: Value) -> Result<(), EvalError> {
        if self.values.contains_key(name) {
            return Err(EvalError::SymbolConflict(name.to_owned()));
        }
        self.values.insert(name.to_owned(), value);
        Ok(())
    }

    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Value> {
        self.values
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_deref()?.resolve(name))
    }
}
