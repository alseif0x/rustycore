//! Pure Difficulty.db2 row schema and row-local flag predicates.

use wow_constants::shared::DifficultyFlags;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyEntry {
    pub id: u32,
    pub instance_type: u8,
    pub flags: u8,
    pub fallback_difficulty_id: u8,
    pub toggle_difficulty_id: u8,
}

impl DifficultyEntry {
    pub fn can_select_like_cpp(&self) -> bool {
        DifficultyFlags::from_bits_truncate(self.flags).contains(DifficultyFlags::CAN_SELECT)
    }

    pub fn is_legacy_like_cpp(&self) -> bool {
        DifficultyFlags::from_bits_truncate(self.flags).contains(DifficultyFlags::LEGACY)
    }
}
