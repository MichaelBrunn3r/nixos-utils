use crate::{eval::Value, map};

pub mod boolean;
pub mod float;
pub mod int;
pub mod list;
pub mod map;
pub mod string;

#[must_use]
pub fn create_module() -> crate::eval::Map<'static> {
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
