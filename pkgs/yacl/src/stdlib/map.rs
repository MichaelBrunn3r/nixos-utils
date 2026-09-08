use crate::eval::Map;

#[must_use]
pub const fn create_map() -> Map<'static> {
    Map::new()
}
