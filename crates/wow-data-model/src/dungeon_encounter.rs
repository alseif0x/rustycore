//! Pure DungeonEncounter.db2 row schema.

/// The localized C++ `Name` field is not part of the current loaded schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonEncounterEntry {
    pub id: u32,
    pub map_id: i16,
    pub difficulty_id: i32,
    pub order_index: i32,
    pub bit: i8,
    pub flags: i32,
    pub faction: i32,
}
