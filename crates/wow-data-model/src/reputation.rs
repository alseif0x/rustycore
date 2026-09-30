//! Shared faction and reputation schema values.

/// C++ `ReputationMgr::_SendStates` spillover fan-out limit.
pub const MAX_SPILLOVER_FACTIONS_LIKE_CPP: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct FactionEntry {
    pub id: u32,
    pub reputation_race_mask: [i64; 4],
    pub reputation_index: i16,
    pub parent_faction_id: u16,
    pub friendship_rep_id: u8,
    pub flags: i32,
    pub paragon_faction_id: u16,
    pub renown_faction_id: i32,
    pub renown_currency_id: i32,
    pub reputation_class_mask: [i16; 4],
    pub reputation_flags: [u16; 4],
    pub reputation_base: [i32; 4],
    pub reputation_max: [i32; 4],
    pub parent_faction_mod: [f32; 2],
    pub parent_faction_cap: [u8; 2],
}

impl FactionEntry {
    pub const fn for_test_like_cpp(id: u32, reputation_index: i16) -> Self {
        Self {
            id,
            reputation_race_mask: [0; 4],
            reputation_index,
            parent_faction_id: 0,
            friendship_rep_id: 0,
            flags: 0,
            paragon_faction_id: 0,
            renown_faction_id: 0,
            renown_currency_id: 0,
            reputation_class_mask: [0; 4],
            reputation_flags: [0; 4],
            reputation_base: [0; 4],
            reputation_max: [0; 4],
            parent_faction_mod: [0.0; 2],
            parent_faction_cap: [0; 2],
        }
    }

    pub const fn can_have_reputation_like_cpp(&self) -> bool {
        self.reputation_index >= 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FriendshipRepReactionEntry {
    pub id: u32,
    pub reaction: String,
    pub friendship_rep_id: u8,
    pub reaction_threshold: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagonReputationEntry {
    pub id: u32,
    pub faction_id: i32,
    pub level_threshold: i32,
    pub quest_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepSpilloverTemplateLikeCpp {
    pub faction: [u32; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
    pub faction_rate: [f32; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
    pub faction_rank: [u8; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
}

impl RepSpilloverTemplateLikeCpp {
    pub const fn empty_like_cpp() -> Self {
        Self {
            faction: [0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
            faction_rate: [0.0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
            faction_rank: [0; MAX_SPILLOVER_FACTIONS_LIKE_CPP],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_entry_preserves_race_masks_flags_and_negative_index_like_cpp() {
        let mut entry = FactionEntry::for_test_like_cpp(7, -1);
        entry.reputation_race_mask = [i64::MIN, -1, 0, i64::MAX];
        entry.reputation_class_mask = [i16::MIN, -1, 0, i16::MAX];
        entry.reputation_flags = [0, 1, 0x8000, u16::MAX];
        entry.flags = i32::MIN;

        assert_eq!(entry.reputation_race_mask, [i64::MIN, -1, 0, i64::MAX]);
        assert_eq!(entry.reputation_class_mask, [i16::MIN, -1, 0, i16::MAX]);
        assert_eq!(entry.reputation_flags, [0, 1, 0x8000, u16::MAX]);
        assert_eq!(entry.flags, i32::MIN);
        assert!(!entry.can_have_reputation_like_cpp());

        entry.reputation_index = 0;
        assert!(entry.can_have_reputation_like_cpp());
    }
}
