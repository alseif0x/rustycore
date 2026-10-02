// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure helpers the session hub (`HubRef`/`HubMut`, `SessionCatalogs` impls) shares with the
//! handlers (#1241 F4a): moved items, no logic change.

use std::collections::{HashMap, HashSet};

use wow_conditions::{QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP};
use wow_constants::Team;

pub(crate) fn player_class_mask(player_class: u8) -> u32 {
    player_class
        .checked_sub(1)
        .and_then(|shift| 1u32.checked_shl(u32::from(shift)))
        .unwrap_or(0)
}

pub(crate) fn player_team_for_race_cpp(race: u8) -> Team {
    match race {
        // C++ resolves this from ChrRacesEntry::Alliance: 1 = Horde, 0 = Alliance.
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => Team::Horde,
        _ => Team::Alliance,
    }
}

/// C++ `SPELL_SCHOOL_MASK_NORMAL` (`SharedDefines.h:329`).
pub(crate) const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: i32 = 1;
/// C++ `SPELL_SCHOOL_MASK_ALL` (`SharedDefines.h:335`): the seven school bits
/// `SpellBaseHealingBonusDone` uses for `ModHealingDonePos`.
pub(crate) const SPELL_SCHOOL_MASK_ALL_LIKE_CPP: i32 = 0x7F;
/// C++ `SPELL_SCHOOL_MASK_SPELL` (`SharedDefines.h:340-343`): fire, nature,
/// frost, shadow and arcane — the full magic mask aura 123 tests against.
pub(crate) const SPELL_SCHOOL_MASK_SPELL_LIKE_CPP: i32 = 0x3E;

/// C++ `CLASSMASK_WAND_USERS` (`SharedDefines.h:190`): the priest, mage and
/// warlock classes ignore the ranged attack power aura producers
/// (`HandleAuraModRangedAttackPower`).
pub(crate) fn class_uses_wands_like_cpp(class: u8) -> bool {
    matches!(class, 5 | 8 | 9)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerGearStatsLikeCpp {
    pub(crate) stats: [i32; 5],
    pub(crate) attack_power: i32,
    pub(crate) ranged_attack_power: i32,
    pub(crate) health: i32,
    pub(crate) mana: i32,
    pub(crate) combat_ratings: [i32; 32],
    pub(crate) spell_power: i32,
    pub(crate) armor: i32,
    pub(crate) resistances: [i32; 7],
    pub(crate) mana_regen_bonus: i32,
    pub(crate) health_regen_bonus: i32,
    pub(crate) spell_penetration_bonus: i32,
    pub(crate) shield_block_base_mod: i32,
    pub(crate) shield_block_value: u32,
    /// C++ `UnitData` weapon ranges installed by `_ApplyWeaponDamage`.
    pub(crate) weapon_damage: [[f32; 2]; 3],
    pub(crate) base_attack_time: [u32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedLootPlayerContext {
    pub(crate) race: u8,
    pub(crate) class: u8,
    pub(crate) gender: u8,
    pub(crate) level: u8,
    pub(crate) known_spells: Vec<i32>,
    pub(crate) active_quest_statuses: HashMap<u32, u8>,
    pub(crate) active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    pub(crate) rewarded_quests: HashSet<u32>,
    pub(crate) inventory_item_counts: HashMap<u32, u32>,
    pub(crate) is_current: bool,
}

impl RepresentedLootPlayerContext {
    pub(crate) fn quest_status(&self, quest_id: u32) -> u8 {
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

    pub(crate) fn inventory_item_count(&self, item_id: u32) -> u32 {
        self.inventory_item_counts
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }
}
