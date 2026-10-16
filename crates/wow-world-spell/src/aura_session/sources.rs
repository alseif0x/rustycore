// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The spell-hit aura-source checks for spell-area autocasts and trait
//! configs, moved from the World `session/spell_state/cast.rs` and
//! `session/spell_state/aura/spell_hit_authority.rs` (#1363 slice 9b).

use std::collections::BTreeSet;

use wow_data::{SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SpellAreaLikeCpp};
use wow_world_core::session::MAX_SPECIALIZATIONS_LIKE_CPP;

use super::AuraHostLikeCpp;

/// Conservative C++ `SpellArea::IsFitToRequirements` projection used only
/// to decide whether an unrepresented AUTOCAST aura could exist. A proven
/// mismatch returns false; battleground and spell-specific conditions are
/// deliberately left as "could fit" because Rust does not own them here.
fn represented_spell_area_could_autocast_for_player_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
    spell_area: &SpellAreaLikeCpp,
) -> bool {
    if spell_area.gender != wow_data::GENDER_NONE_LIKE_CPP
        && spell_area.gender != host.hub_ref().player_gender_like_cpp()
    {
        return false;
    }

    if spell_area.race_mask != 0 {
        let race_mask =
            wow_data::skill::race_mask_for_race_like_cpp(host.hub_ref().player_race_like_cpp());
        if race_mask != 0 && spell_area.race_mask & race_mask as u64 == 0 {
            return false;
        }
    }

    if spell_area.area_id != 0 {
        let Some(world_local) = host.hub_ref().player_world_local_state_like_cpp() else {
            return false;
        };
        if !world_local.has_zone_area_authority_like_cpp() {
            return true;
        }
        let (zone_id, area_id) = world_local.zone_area_like_cpp();
        if spell_area.area_id != zone_id && spell_area.area_id != area_id {
            return false;
        }
    }

    if spell_area.quest_start != 0 {
        let Some(status) =
            host.represented_spell_area_quest_status_like_cpp(spell_area.quest_start)
        else {
            return true;
        };
        let Some(status_mask) = 1_u32.checked_shl(u32::from(status)) else {
            return true;
        };
        if status_mask & spell_area.quest_start_status == 0 {
            return false;
        }
    }

    if spell_area.quest_end != 0 {
        let Some(status) = host.represented_spell_area_quest_status_like_cpp(spell_area.quest_end)
        else {
            return true;
        };
        let Some(status_mask) = 1_u32.checked_shl(u32::from(status)) else {
            return true;
        };
        if status_mask & spell_area.quest_end_status == 0 {
            return false;
        }
    }

    if spell_area.aura_spell != 0 {
        let Some(required_spell_id) = spell_area.aura_spell.checked_abs() else {
            return true;
        };
        let Some(has_aura) = host
            .hub_ref()
            .player_has_visible_aura_spell_like_cpp(required_spell_id)
        else {
            return false;
        };
        if (spell_area.aura_spell > 0 && !has_aura) || (spell_area.aura_spell < 0 && has_aura) {
            return false;
        }
    }

    true
}

pub fn represented_spell_area_autocast_source_is_empty_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
) -> bool {
    host.hub_ref()
        .catalogs
        .spell_catalogs
        .spell_area_store
        .as_ref()
        .is_some_and(|store| {
            store.areas_like_cpp().iter().all(|spell_area| {
                spell_area.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP == 0
                    || !represented_spell_area_could_autocast_for_player_like_cpp(host, spell_area)
            })
        })
}

/// C++ `_LoadTraits` creates missing configs for specialization indexes
/// `0..MAX_SPECIALIZATIONS - 1`, and `CreateTraitConfig` can attach granted
/// entries. This narrow proof is therefore limited to a complete set of
/// persisted, active-for-spec combat configs whose global entry query was
/// empty. `ValidateConfig` accepts those empty configs, so C++ neither
/// replaces them with granted entries nor creates a missing config.
pub fn represented_trait_config_aura_source_is_empty_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
) -> bool {
    let Some(runtime) = crate::player_spell_runtime_snapshot_like_cpp(host) else {
        return false;
    };
    if !runtime.trait_config_rows_complete
        || !runtime.trait_entry_rows_complete
        || !runtime.trait_entry_rows_empty
    {
        return false;
    }

    let Some(specializations) = host.hub_ref().catalogs.chr.specialization_store.as_ref() else {
        return false;
    };
    let mut expected_specs = BTreeSet::new();
    for index in 0..(MAX_SPECIALIZATIONS_LIKE_CPP - 1) {
        let Some(spec) = u8::try_from(index).ok().and_then(|index| {
            specializations
                .get_by_class_and_index_like_cpp(host.hub_ref().player_class_like_cpp(), index)
        }) else {
            return false;
        };
        let Ok(spec_id) = i32::try_from(spec.id) else {
            return false;
        };
        if !expected_specs.insert(spec_id) {
            return false;
        }
    }

    if runtime.trait_config_rows.len() != expected_specs.len() {
        return false;
    }
    for config in runtime.trait_config_rows.values() {
        let (config_type, specialization_id, combat_flags) = config.header;
        const TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP: i32 = 1;
        const TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP: i32 = 0x1;
        if config_type != TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP
            || combat_flags & TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP == 0
            || !expected_specs.remove(&specialization_id)
        {
            return false;
        }
    }
    expected_specs.is_empty()
}
