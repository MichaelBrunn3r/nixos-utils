use std::collections::BTreeMap;

use crate::eval::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Map(BTreeMap<String, Value>);

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

impl Map {
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: &str, value: Value) -> Option<Value> {
        self.0.insert(key.to_owned(), value)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> + '_ {
        self.0.keys().map(String::as_str)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> + '_ {
        self.0.values()
    }
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> From<[(&str, Value); N]> for Map {
    fn from(entries: [(&str, Value); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<'key> FromIterator<(&'key str, Value)> for Map {
    fn from_iter<T: IntoIterator<Item = (&'key str, Value)>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }
}

impl IntoIterator for Map {
    type Item = (String, Value);
    type IntoIter = std::collections::btree_map::IntoIter<String, Value>;

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
