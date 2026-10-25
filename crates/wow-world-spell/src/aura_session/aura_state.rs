// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented aura state reads, the character aura load and the mount
//! display selection, moved from the World `session/spell_state/aura.rs`
//! (#1363 slice 9b).

use std::{collections::BTreeMap, time::Instant};

use rand::seq::SliceRandom;
use wow_core::ObjectGuid;
use wow_data::skill::SKILL_RIDING_LIKE_CPP;
use wow_entities::{
    AuraApplicationLikeCpp as AuraApplication, AuraPersistenceMetadata, AuraThreatSnapshotLikeCpp,
    RepresentedAuraEffectAmountLikeCpp,
};
use wow_world_core::session::{
    AFLAG_NOCASTER_LIKE_CPP, CreatureAggroConfigLikeCpp,
    creature_ai_spell_difficulty_chain_like_cpp,
};
use wow_world_lifecycle::adjusted_represented_pet_aura_remain_time_like_cpp;

use super::{AuraHostLikeCpp, next_player_visible_aura_slot_like_cpp};
use crate::{CharacterAuraEffectRowLikeCpp, CharacterAuraRowLikeCpp};

// C++ SpellAuraDefines.h AURA_FLAGS values used by AuraApplication::_InitFlags
// and BuildUpdatePacket for a restored player aura.
const AFLAG_DURATION: u32 = 0x0000_0004;
const AFLAG_CANCELABLE: u32 = 0x0000_0002;
const AFLAG_NEGATIVE: u32 = 0x0000_0010;
const AFLAG_POSITIVE: u32 = 0x0000_0100;

fn sickness_recalculated_amount(
    effect: &wow_data::SpellEffectInfo,
    base_amount: i32,
    stack_count: u8,
) -> Option<i32> {
    if !matches!(
        effect.effect_aura,
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
            | wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT
            | wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT
    ) {
        return None;
    }
    let mut effect = effect.clone();
    effect.effect_base_points = base_amount;
    effect
        .calc_value_no_caster_like_cpp()
        .checked_mul(i32::from(stack_count))
}

fn ghost_wisp_recalculated_amount(
    spell_id: u32,
    effect_index: u32,
    effect: &wow_data::SpellEffectInfo,
    base_amount: i32,
    stack_count: u8,
) -> Option<i32> {
    let expected_aura = match (spell_id, effect_index) {
        (8_326, 0) => wow_data::spell::aura_types::SPELL_AURA_GHOST,
        (8_326, 1) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        (8_326, 2) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        (20_584, 0) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        (20_584, 1) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        (20_584, 2) => wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        _ => return None,
    };
    if effect.effect_aura != expected_aura {
        return None;
    }
    let mut effect = effect.clone();
    effect.effect_base_points = base_amount;
    effect
        .calc_value_no_caster_like_cpp()
        .checked_mul(i32::from(stack_count))
}

fn selected_ghost_wisp_effect_contract(
    spell_id: u32,
    effects: &[wow_data::SpellEffectInfo],
) -> bool {
    let expected = match spell_id {
        8_326 => [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ],
        20_584 => [
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        ],
        _ => return true,
    };
    if effects.len() != expected.len() {
        return false;
    }
    let mut seen_mask = 0_u32;
    for effect in effects {
        let Some(bit) = 1_u32.checked_shl(effect.effect_index) else {
            return false;
        };
        let Ok(index) = usize::try_from(effect.effect_index) else {
            return false;
        };
        if index >= expected.len()
            || seen_mask & bit != 0
            || effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || effect.effect_aura != expected[index]
            || !matches!(effect.implicit_target_1, 0 | 1)
            || effect.implicit_target_2 != 0
        {
            return false;
        }
        seen_mask |= bit;
    }
    seen_mask == 0b111
}

fn selected_ghost_wisp_can_be_saved(
    caster_guid: ObjectGuid,
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    base_duration_ms: i32,
    attributes: [u32; 15],
    channeled: bool,
) -> bool {
    const SPELL_ATTR0_PASSIVE: u32 = 0x0000_0040;
    const SPELL_ATTR5_LIMIT_N: u32 = 0x0000_0020;
    // This is the selected-loader subset of C++ `Aura::CanBeSaved`. The
    // installed selected effects have self targets and no area-aura targets,
    // and Attr5 covers `SpellInfo::IsSingleTarget`; this loader does not model
    // runtime `Aura::IsSingleTarget` or SpellInfo's custom cannot-save flag.
    attributes[0] & SPELL_ATTR0_PASSIVE == 0
        && !channeled
        && !(caster_guid != player_guid && attributes[5] & SPELL_ATTR5_LIMIT_N != 0)
        && !(item_guid != ObjectGuid::EMPTY && base_duration_ms == -1)
}

fn represented_aura_visual_without_caster_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &CreatureAggroConfigLikeCpp,
) -> u32 {
    let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
        return 0;
    };
    for difficulty_id in creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config) {
        let mut rows = store
            .entries_like_cpp()
            .filter(|row| row.spell_id == spell_id && row.difficulty_id == difficulty_id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        // `SpellMgr` inserts DB2 rows at `lower_bound` ordered by descending
        // CasterPlayerConditionID. Because the DB2 store is walked in ID
        // order, equal-condition rows are inserted before earlier rows and
        // therefore end up in descending DB2-ID order.
        rows.sort_by_key(|row| std::cmp::Reverse((row.caster_player_condition_id, row.id)));
        return rows
            .into_iter()
            .find(|row| row.caster_player_condition_id == 0 && row.caster_unit_condition_id == 0)
            .map_or(0, |row| row.id);
    }
    0
}

pub fn represented_player_aura_state_mask_like_cpp<H: AuraHostLikeCpp>(host: &H) -> Option<u32> {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_player_aura_state_mask_like_cpp(hub)
}

/// C++ `Unit::m_unitData->AuraState` for any represented unit: the canonical
/// session player or a world creature. `0` when the unit cannot be resolved.
pub fn represented_unit_aura_state_mask_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
    unit_guid: ObjectGuid,
) -> u32 {
    if Some(unit_guid) == host.player_guid() {
        return represented_player_aura_state_mask_like_cpp(host).unwrap_or(0);
    }
    host.core()
        .with_represented_creature_like_cpp(unit_guid, |creature| {
            creature.unit().subsystems().auras.aura_state_mask
                | wow_entities::Creature::health_aura_state_like_cpp(
                    u64::from(creature.current_hp()),
                    u64::from(creature.max_hp()),
                    creature.is_alive(),
                )
        })
        .unwrap_or(0)
}

pub(crate) fn calculate_represented_mounted_aura_amount_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
    spell_id: i32,
    effect: &wow_data::SpellEffectInfo,
) -> i32 {
    let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
    if let Some(mount_entry) = host
        .hub_ref()
        .catalogs
        .mount_store
        .as_ref()
        .and_then(|store| {
            u32::try_from(spell_id)
                .ok()
                .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
        })
    {
        mount_type_id = mount_entry.mount_type_id;
    }

    if mount_type_id != 0 {
        let Some(riding_skill) = host
            .hub_ref()
            .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)
        else {
            return effect.effect_base_points;
        };
        if let Some((is_submerged, is_in_water)) = host
            .hub_ref()
            .represented_player_mount_liquid_state_like_cpp()
            && let Ok(capability) = crate::represented_mount_capability_selection_for_type_like_cpp(
                host,
                mount_type_id,
                u32::from(riding_skill),
                None,
                is_submerged,
                is_in_water,
            )
        {
            return i32::try_from(capability.id).unwrap_or(effect.effect_base_points);
        }
    }

    effect.effect_base_points
}

pub fn load_represented_character_auras_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    aura_rows: impl IntoIterator<Item = CharacterAuraRowLikeCpp>,
    effect_rows: impl IntoIterator<Item = CharacterAuraEffectRowLikeCpp>,
    timediff_secs: u32,
) -> usize {
    let Some(player_guid) = host.player_guid() else {
        crate::set_player_aura_authority_complete_like_cpp(host, false);
        return 0;
    };
    let spell_store = host.hub_ref().catalogs.spell_store().cloned();
    let difficulty_store = host.hub_ref().catalogs.difficulty_store().cloned();
    let aura_options_store = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_aura_options_store
        .clone();
    let spell_duration_store = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_duration_store
        .clone();
    let spell_misc_store = host
        .hub_ref()
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .cloned();
    let selected_sickness_spell_id = {
        let hub = host.hub_ref();
        hub.catalogs
            .chr
            .races_store
            .as_ref()
            .and_then(|store| store.get(u32::from(hub.player_race_like_cpp())))
            .and_then(|race| u32::try_from(race.res_sickness_spell_id).ok())
    };

    let mut authority_complete = true;
    let mut effects = Vec::new();
    for row in effect_rows {
        let selected_ghost_wisp = matches!(row.spell_id, 8_326 | 20_584);
        if row.spell_id == 0
            || (!selected_ghost_wisp && row.effect_mask == 0)
            || (!selected_ghost_wisp
                && u32::from(row.effect_index) >= wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP)
        {
            authority_complete = false;
            continue;
        }
        effects.push(row);
    }

    let mut loaded = 0usize;
    for mut row in aura_rows {
        let selected_ghost_wisp = matches!(row.spell_id, 8_326 | 20_584);
        let Ok(spell_id) = i32::try_from(row.spell_id) else {
            authority_complete = false;
            continue;
        };
        if row.spell_id == 0
            || (!selected_ghost_wisp && row.effect_mask == 0)
            || row.stack_count == 0
        {
            authority_complete = false;
            continue;
        }
        let Some(spell_store) = spell_store.as_deref() else {
            authority_complete = false;
            continue;
        };
        let Some(spell_info) = spell_store.get(spell_id) else {
            authority_complete = false;
            continue;
        };
        let difficulty_store_ref = if row.difficulty == 0 {
            None
        } else {
            let Some(store) = difficulty_store.as_deref() else {
                authority_complete = false;
                continue;
            };
            if !store.contains(u32::from(row.difficulty)) {
                authority_complete = false;
                continue;
            }
            Some(store)
        };
        let spell_effects = spell_store.effects_for_difficulty_like_cpp(
            spell_id,
            row.difficulty,
            difficulty_store_ref,
        );
        if spell_effects.is_none() {
            authority_complete = false;
        }
        let spell_effects = spell_effects.unwrap_or(&[]);
        if selected_ghost_wisp && !selected_ghost_wisp_effect_contract(row.spell_id, spell_effects)
        {
            authority_complete = false;
            continue;
        }
        let active_effect_mask = if selected_ghost_wisp {
            row.effect_mask & 0b111
        } else {
            row.effect_mask
        };

        let selected_metadata = if selected_ghost_wisp {
            if aura_options_store.is_none() {
                authority_complete = false;
                continue;
            }
            let Some(spell_misc_store) = spell_misc_store.as_ref() else {
                authority_complete = false;
                continue;
            };
            let Some(misc) = spell_misc_store.entry_for_spell_difficulty_with_fallback_like_cpp(
                row.spell_id,
                row.difficulty,
                difficulty_store_ref,
            ) else {
                authority_complete = false;
                continue;
            };
            let Some(spell_duration_store) = spell_duration_store.as_deref() else {
                authority_complete = false;
                continue;
            };
            let base_duration_ms = wow_data::spell_duration_ms_like_cpp(
                u32::from(misc.duration_index),
                Some(spell_duration_store),
            );
            if base_duration_ms != -1 {
                authority_complete = false;
                continue;
            }
            let Some(attributes) = spell_store.misc_attributes_for_difficulty_like_cpp(
                spell_id,
                row.difficulty,
                difficulty_store_ref,
            ) else {
                authority_complete = false;
                continue;
            };
            Some((
                attributes,
                base_duration_ms,
                spell_store.is_channeled_like_cpp(spell_id),
            ))
        } else {
            None
        };

        let aura_expires_offline = spell_misc_store
            .as_ref()
            .and_then(|store| store.get_by_spell_id(row.spell_id))
            .is_some_and(|misc| {
                (misc.attributes[4] as u32
                    & wow_data::spell::attributes::SPELL_ATTR4_AURA_EXPIRES_OFFLINE)
                    != 0
            });
        let Some(remain_time_ms) = adjusted_represented_pet_aura_remain_time_like_cpp(
            row.remain_time_ms,
            timediff_secs,
            wow_data::represented_spell_is_positive_like_cpp(spell_info),
            aura_expires_offline,
        ) else {
            // An aura that expires during logout is a legitimate loaded absence.
            continue;
        };
        row.remain_time_ms = remain_time_ms;

        if let Some(store) = aura_options_store.as_ref() {
            let proc_charges = store.proc_charges_like_cpp(row.spell_id, row.difficulty);
            row.remain_charges = if proc_charges == 0 {
                0
            } else if row.remain_charges == 0 {
                proc_charges
            } else {
                row.remain_charges
            };
        }
        let selected_cast_id = if selected_ghost_wisp {
            let Some(cast_id) = host.next_represented_spell_cast_guid_like_cpp(spell_id) else {
                authority_complete = false;
                break;
            };
            Some(cast_id)
        } else {
            None
        };
        if selected_ghost_wisp && active_effect_mask == 0 {
            // The selected SpellInfo and charges are known, so this is a
            // source-equivalent TryCreate absence after allocating its CastId.
            continue;
        }

        let caster_guid = row.caster_guid;
        let mut selected_metadata_valid = true;
        let mut keyed_effect_rows = BTreeMap::new();
        for effect in effects.iter().filter(|effect| {
            effect.caster_guid == row.caster_guid
                && effect.item_guid == row.item_guid
                && effect.spell_id == row.spell_id
                && effect.effect_mask == row.effect_mask
        }) {
            if selected_ghost_wisp {
                let effect_bit = 1_u32.checked_shl(u32::from(effect.effect_index));
                if effect_bit.is_none_or(|bit| active_effect_mask & bit == 0) {
                    continue;
                }
            }
            if keyed_effect_rows
                .insert(effect.effect_index, *effect)
                .is_some()
            {
                authority_complete = false;
                if selected_ghost_wisp {
                    selected_metadata_valid = false;
                }
            }
        }
        if !selected_ghost_wisp && row.recalculate_mask & !row.effect_mask != 0 {
            authority_complete = false;
        }
        let mut recalculate_mask = if selected_ghost_wisp {
            row.recalculate_mask & active_effect_mask
        } else {
            row.recalculate_mask
        };
        let mut covered_mask = 0_u32;
        let mut canonical_effects = Vec::new();
        let mut represented_effect_amounts = Vec::new();
        let mut base_amounts = Vec::new();
        for effect in spell_effects {
            let Some(bit) = 1_u32.checked_shl(effect.effect_index) else {
                continue;
            };
            if active_effect_mask & bit == 0 {
                continue;
            }
            let Ok(effect_index) = u8::try_from(effect.effect_index) else {
                authority_complete = false;
                continue;
            };
            if !effect.is_aura_like_cpp() || covered_mask & bit != 0 {
                authority_complete = false;
                continue;
            }
            covered_mask |= bit;
            let saved_effect = keyed_effect_rows.get(&effect_index);
            if saved_effect.is_none() {
                authority_complete = false;
                if selected_ghost_wisp {
                    selected_metadata_valid = false;
                }
            }
            let base_amount = saved_effect.map_or(0, |saved| saved.base_amount);
            let stored_amount = saved_effect.map_or(0, |saved| saved.amount);
            let loaded_transform_disables_recalculation = selected_ghost_wisp
                && row.spell_id == 20_584
                && effect.effect_index == 2
                && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM;
            let current_amount =
                if recalculate_mask & bit == 0 {
                    stored_amount
                } else if selected_sickness_spell_id == Some(row.spell_id) {
                    match sickness_recalculated_amount(effect, base_amount, row.stack_count) {
                        Some(amount) => amount,
                        None => {
                            authority_complete = false;
                            if selected_ghost_wisp {
                                selected_metadata_valid = false;
                            }
                            stored_amount
                        }
                    }
                } else if let Some(amount) = ghost_wisp_recalculated_amount(
                    row.spell_id,
                    effect.effect_index,
                    effect,
                    base_amount,
                    row.stack_count,
                ) {
                    if loaded_transform_disables_recalculation {
                        // C++ `CalculateAmount` recalculates Transform from
                        // base points, then clears `CanBeRecalculated`.
                        recalculate_mask &= !bit;
                    }
                    amount
                } else {
                    // Preserve the stored amount for unsupported recalculation
                    // metadata while withholding complete aura authority.
                    authority_complete = false;
                    stored_amount
                };
            if let Some(saved_effect) = saved_effect {
                base_amounts.push((effect_index, saved_effect.base_amount));
            }
            represented_effect_amounts.push(RepresentedAuraEffectAmountLikeCpp {
                effect_index,
                amount: current_amount,
            });
            canonical_effects.push((
                bit,
                effect.effect_aura,
                current_amount,
                effect.effect_misc_value_1,
            ));
        }
        if covered_mask != active_effect_mask {
            authority_complete = false;
            if selected_ghost_wisp {
                selected_metadata_valid = false;
            }
        }
        if selected_ghost_wisp && !selected_metadata_valid {
            continue;
        }

        let interrupt_flags = spell_store
            .aura_interrupt_flags_for_difficulty_like_cpp(
                spell_id,
                row.difficulty,
                difficulty_store_ref,
            )
            .unwrap_or([0; 2]);
        let canonical_snapshot = AuraThreatSnapshotLikeCpp::new(interrupt_flags, canonical_effects);
        let duration_total = u32::try_from(row.max_duration_ms).unwrap_or(0);
        let duration_remaining = u32::try_from(row.remain_time_ms).unwrap_or(0);
        // C++ `_LoadAuras` allocates this before `TryCreate`, including a
        // selected aura whose active effect mask later normalizes to zero.
        let Some(cast_id) =
            selected_cast_id.or_else(|| host.next_represented_spell_cast_guid_like_cpp(spell_id))
        else {
            authority_complete = false;
            break;
        };
        let spell_visual_id = represented_aura_visual_without_caster_like_cpp(
            row.spell_id,
            row.difficulty,
            &host.hub_ref().config.creature_aggro_config_like_cpp,
        );
        let provenance = wow_entities::AuraCastProvenanceLikeCpp {
            cast_id,
            spell_visual_id: i32::try_from(spell_visual_id).unwrap_or(i32::MAX),
        };
        let selected_positive = if selected_ghost_wisp {
            match super::selected_aura_effects_positive(spell_info, active_effect_mask) {
                Some(positive) => positive,
                None => {
                    authority_complete = false;
                    continue;
                }
            }
        } else {
            wow_data::represented_spell_is_positive_like_cpp(spell_info)
        };
        // The selected loader derives polarity from active effects. C++ also
        // uses a live foreign caster's friendliness here; this load path does
        // not resolve that caster, so foreign polarity is not parity evidence.
        let mut aura_flags = if selected_positive {
            AFLAG_POSITIVE
        } else {
            AFLAG_NEGATIVE
        };
        if selected_ghost_wisp && selected_positive {
            let Some((attributes, _, _)) = selected_metadata else {
                authority_complete = false;
                continue;
            };
            if attributes[0] as u32 & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE == 0
                && attributes[1] as u32 & wow_data::spell::attributes::SPELL_ATTR1_NO_AURA_ICON == 0
            {
                // The aura owner and target are this Player, regardless of
                // the persisted caster GUID.
                aura_flags |= AFLAG_CANCELABLE;
            }
        }
        if caster_guid == player_guid {
            aura_flags |= AFLAG_NOCASTER_LIKE_CPP;
        }
        if row.max_duration_ms > 0 {
            aura_flags |= AFLAG_DURATION;
        }

        if selected_ghost_wisp {
            let Some((attributes, base_duration_ms, channeled)) = selected_metadata else {
                authority_complete = false;
                continue;
            };
            // `_LoadAuras` uses `Aura::TryCreate`; every persisted row creates
            // a new Aura and reaches `_AddAura` conflict removal. The refresh
            // path belongs to fresh cast application, not login loading.
            let Some(replaced_slots) = host.core().with_owned_player_like_cpp(|player| {
                let auras = &player.unit().subsystems().auras;
                auras.ordered_slots_for_spell_removal(spell_id)
            }) else {
                authority_complete = false;
                continue;
            };
            let Ok(replaced_slots) = replaced_slots else {
                authority_complete = false;
                continue;
            };
            let mut removals_succeeded = true;
            for slot in replaced_slots {
                if host.remove_aura(slot).is_err() {
                    authority_complete = false;
                    removals_succeeded = false;
                    break;
                }
            }
            if !removals_succeeded {
                continue;
            }
            if !selected_ghost_wisp_can_be_saved(
                caster_guid,
                player_guid,
                row.item_guid,
                base_duration_ms,
                attributes,
                channeled,
            ) {
                // `_AddAura` conflict removal precedes `CanBeSaved`; the new
                // aura is then discarded without applying its effects.
                continue;
            }
        }
        let Some(slot) = next_player_visible_aura_slot_like_cpp(host) else {
            authority_complete = false;
            break;
        };
        let aura = AuraApplication {
            spell_id,
            difficulty_id: row.difficulty,
            caster_guid,
            slot,
            duration_total,
            duration_remaining,
            stack_count: row.stack_count,
            aura_flags,
            effect_mask: active_effect_mask,
            aura_interrupt_flags: interrupt_flags[0],
            aura_interrupt_flags2: interrupt_flags[1],
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts,
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };
        let persistence_metadata = AuraPersistenceMetadata {
            item_guid: row.item_guid,
            cast_item_id: row.cast_item_id,
            cast_item_level: row.cast_item_level,
            recalculate_mask,
            charges: row.remain_charges,
            base_amounts,
        };
        let selected_ghost_wisp_application =
            matches!(row.spell_id, 8326 | 20584).then(|| aura.clone());
        let _fallback_snapshot = canonical_snapshot.clone();
        let _fallback_aura = aura.clone();
        let _fallback_metadata = persistence_metadata.clone();
        let _canonical = host.core().with_owned_player_mut_like_cpp(|player| {
            player.install_player_threat_aura_like_cpp(slot, canonical_snapshot.clone(), aura);
            let metadata_installed = player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_aura_persistence_metadata(slot, persistence_metadata);
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_aura_cast_provenance_like_cpp(slot, provenance);
            player.apply_player_threat_aura_like_cpp(
                row.spell_id,
                caster_guid,
                slot,
                canonical_snapshot,
            );
            metadata_installed
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        let installed = if _canonical.is_some() {
            if _canonical == Some(false) {
                authority_complete = false;
            }
            Some(())
        } else if host.test_fixtures_ref_like_cpp().is_some()
            && host.core().player_handle_like_cpp.is_none()
        {
            let (state, mut hub) = host.split_spell_state_mut();
            let metadata_installed =
                state.mutate_player_aura_subsystem_like_cpp(&mut hub, |auras| {
                    auras.insert_threat_snapshot_like_cpp(slot, _fallback_snapshot);
                    auras.insert_runtime_application_like_cpp(_fallback_aura);
                    auras.set_aura_cast_provenance_like_cpp(slot, provenance);
                    let metadata_installed =
                        auras.set_aura_persistence_metadata(slot, _fallback_metadata);
                    let effects = auras
                        .threat_snapshot_like_cpp(slot)
                        .map(|snapshot| snapshot.effects().to_vec())
                        .unwrap_or_default();
                    for (bit, aura_type, amount, misc_value) in effects {
                        let applied =
                            wow_entities::AppliedAuraRef::new(row.spell_id, caster_guid, slot, bit);
                        auras.register_applied_aura(
                            applied,
                            None,
                            interrupt_flags[0],
                            interrupt_flags[1],
                        );
                        auras.register_applied_aura_effect_like_cpp(
                            applied, aura_type, amount, misc_value,
                        );
                    }
                    metadata_installed
                });
            if metadata_installed == Some(false) {
                authority_complete = false;
            }
            metadata_installed.map(|_| ())
        } else {
            None
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let installed = _canonical.filter(|metadata_installed| *metadata_installed);
        if installed.is_none() {
            authority_complete = false;
        }
        if _canonical.is_none() && installed.is_none() {
            authority_complete = false;
            break;
        }
        if let Some(aura) = selected_ghost_wisp_application
            && host.apply_loaded_ghost_wisp_aura(&aura).is_err()
        {
            authority_complete = false;
        }
        loaded += 1;
    }
    if !authority_complete {
        crate::set_player_aura_authority_complete_like_cpp(host, false);
    }
    loaded
}

#[allow(dead_code)]
pub fn represented_mount_aura_display_candidates_like_cpp<H: AuraHostLikeCpp>(
    host: &H,
    spell_id: u32,
) -> Vec<i32> {
    let Some(mount) = host
        .hub_ref()
        .catalogs
        .mount_store
        .as_ref()
        .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
    else {
        return Vec::new();
    };

    if mount.flags & wow_data::MOUNT_FLAG_SELF_MOUNT != 0 {
        return vec![wow_data::DISPLAYID_HIDDEN_MOUNT];
    }

    let Some(displays) = host
        .hub_ref()
        .catalogs
        .mount_x_display_store
        .as_ref()
        .and_then(|store| store.displays_for_mount_like_cpp(mount.id))
    else {
        return Vec::new();
    };

    displays
        .iter()
        .filter(|display| {
            display.player_condition_id == 0
                || host.represented_mount_x_display_usable_like_cpp(display.player_condition_id)
        })
        .map(|display| display.creature_display_info_id)
        .collect()
}

#[allow(dead_code)]
pub(crate) fn select_represented_mount_aura_display_like_cpp<H: AuraHostLikeCpp>(
    host: &mut H,
    spell_id: u32,
) -> Option<i32> {
    let candidates = represented_mount_aura_display_candidates_like_cpp(host, spell_id);
    candidates
        .choose(&mut host.core_mut().driver.represented_runtime_rng_like_cpp)
        .copied()
}
