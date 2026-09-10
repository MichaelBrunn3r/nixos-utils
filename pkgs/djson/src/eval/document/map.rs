use std::collections::BTreeMap;

use crate::eval::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Map<'input>(BTreeMap<&'input str, Value<'input>>);

#[macro_export]
macro_rules! map {
    () => {
        $crate::eval::document::map::Map::new()
    };
    ($($key:literal : $value:expr),+ $(,)?) => {
        $crate::eval::document::map::Map::from([$(($key, $crate::value!($value))),+])
    };
    ($($key:ident : $value:expr),+ $(,)?) => {
        $crate::eval::document::map::Map::from([$( (stringify!($key), $crate::value!($value)) ),+])
    };
}

impl<'input> Map<'input> {
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value<'input>> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: &'input str, value: Value<'input>) -> Option<Value<'input>> {
        self.0.insert(key, value)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Map<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'input, const N: usize> From<[(&'input str, Value<'input>); N]> for Map<'input> {
    fn from(entries: [(&'input str, Value<'input>); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<'input> FromIterator<(&'input str, Value<'input>)> for Map<'input> {
    fn from_iter<T: IntoIterator<Item = (&'input str, Value<'input>)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'input> IntoIterator for Map<'input> {
    type Item = (&'input str, Value<'input>);
    type IntoIter = std::collections::btree_map::IntoIter<&'input str, Value<'input>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::Map;
    use crate::eval::Value;

    #[test]
    fn map_macro_constructs_map() {
        let map = map! {
            "answer": 42,
            "enabled": true,
        };

        assert_eq!(map.get("answer"), Some(&Value::Int(42)));
        assert_eq!(map.get("enabled"), Some(&Value::Bool(true)));
    }

    #[test]
    fn map_macro_constructs_empty_map() {
        assert_eq!(map!(), Map::new());
    }

    #[test]
    fn map_macro_stringifies_identifier_keys() {
        let map = map! {
            answer: 42,
        };

        assert_eq!(map.get("answer"), Some(&Value::Int(42)));
    }
}
