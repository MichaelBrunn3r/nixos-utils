use crate::{
    eval::{Map, Value},
    map,
};

pub mod boolean;
pub mod float;
pub mod int;
pub mod list;
pub mod map;
pub mod string;

#[must_use]
pub fn create_module() -> Map {
    map! {
        bool: Value::Map(boolean::create_map()),
        float: Value::Map(float::create_map()),
        int: Value::Map(int::create_map()),
        list: Value::Map(list::create_map()),
        map: Value::Map(map::create_map()),
        none: Value::Map(Map::new()),
        str: Value::Map(string::create_map()),
    }
}
