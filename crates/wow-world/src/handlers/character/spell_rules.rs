// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure spell, skill, and cooldown projections used during character login.

use std::collections::{BTreeMap, HashMap, HashSet};

use wow_packet::packets::misc::{SpellChargeEntry, SpellHistoryEntry};

use crate::session::RepresentedPlayerSkillLikeCpp;

pub(super) fn active_known_spell_for_send_like_cpp(
    spell_id: u32,
    active: u8,
    disabled: u8,
) -> Option<i32> {
    if spell_id > 0 && active != 0 && disabled == 0 {
        i32::try_from(spell_id).ok()
    } else {
        None
    }
}

pub(super) fn loaded_spell_for_add_spell_side_effects_like_cpp(
    spell_id: u32,
    disabled: u8,
) -> Option<i32> {
    if spell_id > 0 && disabled == 0 {
        i32::try_from(spell_id).ok()
    } else {
        None
    }
}

pub(super) fn apply_skill_rewarded_spell_changes_to_login_like_cpp(
    known_spells: &mut Vec<i32>,
    loaded_spell_side_effect_spells: &mut Vec<i32>,
    dependent_spells: &mut HashSet<i32>,
    removed_spells: &mut HashSet<i32>,
    changes: wow_data::SkillRewardedSpellChangesLikeCpp,
) {
    for spell_id in changes.remove {
        known_spells.retain(|known_spell| *known_spell != spell_id);
        loaded_spell_side_effect_spells.retain(|known_spell| *known_spell != spell_id);
        dependent_spells.remove(&spell_id);
        removed_spells.insert(spell_id);
    }
    for spell_id in changes.learn {
        removed_spells.remove(&spell_id);
        if !known_spells.contains(&spell_id) {
            known_spells.push(spell_id);
        }
        if !loaded_spell_side_effect_spells.contains(&spell_id) {
            loaded_spell_side_effect_spells.push(spell_id);
        }
        dependent_spells.insert(spell_id);
    }
}

pub(in crate::handlers::character) const SKILL_UNARMED_LIKE_CPP: u16 = 162;
pub(in crate::handlers::character) const SKILL_FIST_WEAPONS_LIKE_CPP: u16 = 473;

/// Pinned 3.4.3 C++ `Player::_LoadSkills` final Fist Weapons fixup.
///
/// `HasSkill(SKILL_FIST_WEAPONS)` is false for a zero-rank loaded row, so only
/// an active persisted Fist Weapons skill is synchronized. `SetSkill` removes
/// it when Unarmed is absent/zero; otherwise it copies the current Unarmed
/// value and the level-dependent maximum.
pub(super) fn sync_loaded_fist_weapons_with_unarmed_like_cpp(
    skill_records: &mut HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    skill_info_by_id: &mut BTreeMap<u16, wow_data::SkillInfoEntry>,
    level: u8,
) {
    let Some(mut fist_weapons) = skill_info_by_id
        .get(&SKILL_FIST_WEAPONS_LIKE_CPP)
        .copied()
        .filter(|entry| entry.rank > 0)
    else {
        return;
    };

    let unarmed_rank = skill_info_by_id
        .get(&SKILL_UNARMED_LIKE_CPP)
        .map(|entry| entry.rank)
        .unwrap_or(0);
    fist_weapons.step = 0;
    fist_weapons.rank = unarmed_rank;
    fist_weapons.max_rank = if unarmed_rank == 0 {
        0
    } else {
        u16::from(level).saturating_mul(5)
    };
    fist_weapons.temp_bonus = 0;
    fist_weapons.perm_bonus = 0;
    skill_info_by_id.insert(SKILL_FIST_WEAPONS_LIKE_CPP, fist_weapons);

    if unarmed_rank == 0 {
        // C++ `SetSkill(..., newVal = 0, ...)` marks the persisted skill
        // deleted while leaving its cleared initial update-field slot.
        skill_records.remove(&SKILL_FIST_WEAPONS_LIKE_CPP);
    } else if let Some(skill_record) = skill_records.get_mut(&SKILL_FIST_WEAPONS_LIKE_CPP) {
        skill_record.value = fist_weapons.rank;
        skill_record.max = fist_weapons.max_rank;
    }
}

pub(super) fn favorite_known_spells_for_send_like_cpp(
    known_spells: &[i32],
    favorite_spells: &HashSet<i32>,
) -> Vec<i32> {
    known_spells
        .iter()
        .copied()
        .filter(|spell_id| favorite_spells.contains(spell_id))
        .collect()
}

pub(super) fn unix_now_secs_like_cpp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn remaining_ms_from_unix_secs_like_cpp(end_unix_secs: i64, now_unix_secs: i64) -> Option<u32> {
    let remaining_secs = end_unix_secs.checked_sub(now_unix_secs)?;
    if remaining_secs <= 0 {
        return None;
    }

    u32::try_from(remaining_secs.saturating_mul(1000)).ok()
}

pub(super) fn spell_history_entry_from_db_like_cpp(
    spell_id: u32,
    item_id: u32,
    cooldown_end_unix_secs: i64,
    category_id: u32,
    category_end_unix_secs: i64,
    now_unix_secs: i64,
) -> Option<SpellHistoryEntry> {
    let cooldown_ms = remaining_ms_from_unix_secs_like_cpp(cooldown_end_unix_secs, now_unix_secs)?;
    let category_ms =
        remaining_ms_from_unix_secs_like_cpp(category_end_unix_secs, now_unix_secs).unwrap_or(0);

    Some(SpellHistoryEntry {
        spell_id,
        item_id,
        category: if category_ms > 0 { category_id } else { 0 },
        recovery_time_ms: if cooldown_ms > category_ms {
            cooldown_ms as i32
        } else {
            0
        },
        category_recovery_time_ms: category_ms as i32,
        mod_rate: 1.0,
        on_hold: false,
    })
}

pub(super) fn spell_charge_entry_from_db_like_cpp(
    category_id: u32,
    first_recharge_end_unix_secs: i64,
    consumed_charges: u8,
    now_unix_secs: i64,
) -> Option<SpellChargeEntry> {
    let next_recovery_time_ms =
        remaining_ms_from_unix_secs_like_cpp(first_recharge_end_unix_secs, now_unix_secs)?;
    Some(SpellChargeEntry {
        category: category_id,
        next_recovery_time_ms,
        charge_mod_rate: 1.0,
        consumed_charges,
    })
}
