use std::{collections::BTreeMap, rc::Rc};

use crate::eval::{EvalError, Value};

#[derive(Clone, Debug)]
pub struct Scope<'input> {
    values: BTreeMap<&'input str, Value<'input>>,
    parent: Option<Rc<Self>>,
}

impl<'input> Scope<'input> {
    #[must_use]
    pub fn from_values(values: impl IntoIterator<Item = (&'input str, Value<'input>)>) -> Self {
        Self {
            values: values.into_iter().collect(),
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
    pub fn bind_value(&mut self, name: &'input str, value: Value<'input>) -> Result<(), EvalError> {
        if self.values.contains_key(name) {
            return Err(EvalError::SymbolConflict(name.to_owned()));
        }
        self.values.insert(name, value);
        Ok(())
    }

    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Value<'input>> {
        self.values
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_deref()?.resolve(name))
    }
}
