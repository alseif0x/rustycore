//! Represented player facts the loot adapters read: class, race, team, quest status,
//! faction rules and enchanting skill.
//!
//! Split out of `loot/mod.rs` under #584 (B5); items are unchanged.

use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::handlers::loot) struct ItemTemplateAddonLootMetadataLikeCpp {
    pub(in crate::handlers::loot) flags_cu: u32,
    pub(in crate::handlers::loot) quest_log_item_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::handlers::loot) struct RepresentedLootPlayerContext {
    pub(in crate::handlers::loot) race: u8,
    pub(in crate::handlers::loot) class: u8,
    pub(in crate::handlers::loot) gender: u8,
    pub(in crate::handlers::loot) level: u8,
    pub(in crate::handlers::loot) known_spells: Vec<i32>,
    pub(in crate::handlers::loot) active_quest_statuses: HashMap<u32, u8>,
    pub(in crate::handlers::loot) active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    pub(in crate::handlers::loot) rewarded_quests: HashSet<u32>,
    pub(in crate::handlers::loot) inventory_item_counts: HashMap<u32, u32>,
    pub(in crate::handlers::loot) is_current: bool,
}

impl RepresentedLootPlayerContext {
    pub(in crate::handlers::loot) fn quest_status(&self, quest_id: u32) -> u8 {
        self.active_quest_statuses
            .get(&quest_id)
            .copied()
            .or_else(|| {
                self.rewarded_quests
                    .contains(&quest_id)
                    .then_some(QUEST_STATUS_REWARDED_LIKE_CPP)
            })
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
    }

    pub(in crate::handlers::loot) fn inventory_item_count(&self, item_id: u32) -> u32 {
        self.inventory_item_counts
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }
}

impl ItemTemplateAddonLootMetadataLikeCpp {
    pub(in crate::handlers::loot) fn ignores_quest_status(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP != 0
    }

    pub(in crate::handlers::loot) fn follows_loot_rules(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP != 0
    }
}

pub(in crate::handlers::loot) fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    if (1..=13).contains(&class_id) {
        Some(1_u32 << (class_id - 1))
    } else {
        None
    }
}

pub(in crate::handlers::loot) fn player_race_mask_like_cpp(race_id: u8) -> Option<u32> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u32 << bit)
}

pub(in crate::handlers::loot) fn player_team_for_race_cpp_representable(race: u8) -> u32 {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => 67,
        _ => 469,
    }
}

pub(in crate::handlers::loot) fn represented_item_faction_flags_block_player_like_cpp(flags2: Option<u32>, race: u8) -> bool {
    let Some(flags2) = flags2 else {
        return false;
    };

    let team = player_team_for_race_cpp_representable(race);
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team != 67)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team != 469)
}

pub(in crate::handlers::loot) fn player_quest_status_mask_like_cpp(status: Option<u8>, rewarded: bool) -> u32 {
    if rewarded {
        return 0x40;
    }

    match status {
        None => 0x01,
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => 0x02,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => 0x08,
        Some(QUEST_STATUS_FAILED_LIKE_CPP) => 0x20,
        _ => 0,
    }
}

pub(in crate::handlers::loot) fn loot_type_for_client_like_cpp(loot_type: u8) -> u8 {
    match loot_type {
        LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP => {
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        }
        LOOT_TYPE_INSIGNIA_LIKE_CPP => LOOT_TYPE_SKINNING_LIKE_CPP,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP | LOOT_TYPE_FISHING_JUNK_LIKE_CPP => {
            LOOT_TYPE_FISHING_LIKE_CPP
        }
        _ => loot_type,
    }
}

pub(in crate::handlers::loot) fn represented_max_enchanting_skill_like_cpp(
    looters: &[ObjectGuid],
    current_player_guid: ObjectGuid,
    current_player_enchanting_skill: Option<u16>,
    player_registry: Option<&PlayerRegistry>,
) -> u16 {
    looters
        .iter()
        .filter_map(|looter| {
            if *looter == current_player_guid {
                current_player_enchanting_skill
            } else {
                player_registry.and_then(|registry| registry.loot_enchanting_skill(*looter))
            }
        })
        .max()
        .unwrap_or(0)
}
