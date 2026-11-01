// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented pet loading: the login pet rows, the pet mode state and the
//! loaded pet aura remain time (#1363 slice 7a; moved from the World
//! `session/pet_loading.rs` and `session/pets/persistence.rs`). The former
//! `PetsCx` methods take the same lifecycle/hub split from the host at their
//! first statement, exactly where the World built the context.

use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::UnitMoveTypeLikeCpp;
use wow_world_core::session::react_state_from_db_like_cpp;

use super::PetSessionHostLikeCpp;
use crate::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
};

pub fn adjusted_represented_pet_aura_remain_time_like_cpp(
    remain_time_ms: i32,
    timediff_secs: u32,
    is_positive: bool,
    aura_expires_offline: bool,
) -> Option<i32> {
    if remain_time_ms != -1 && (!is_positive || aura_expires_offline) {
        let timediff_secs = i32::try_from(timediff_secs).unwrap_or(i32::MAX);
        if remain_time_ms / 1_000 <= timediff_secs {
            return None;
        }

        return Some(remain_time_ms.saturating_sub(timediff_secs.saturating_mul(1_000)));
    }

    Some(remain_time_ms)
}

pub fn set_represented_pet_mode_state_with_spell_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_guid: Option<ObjectGuid>,
    react_state: u8,
    command_state: u8,
    created_by_spell: u32,
) {
    if !host.hub_mut().set_player_pet_guid_like_cpp(pet_guid) {
        return;
    }
    host.hub_mut()
        .invalidate_represented_character_pet_empty_authority_like_cpp();
    let canonical = pet_guid.is_some_and(|pet_guid| {
        host.core()
            .with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
                pet.set_created_by_spell_id_like_cpp(created_by_spell);
                pet.creature_mut()
                    .set_react_state(react_state_from_db_like_cpp(react_state));
                pet.creature_mut()
                    .unit_mut()
                    .subsystems_mut()
                    .control
                    .init_charm_info()
                    .command_state = command_state;
            })
            .is_some()
    });
    #[cfg(any(test, feature = "test-fixtures"))]
    if !canonical && let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.pets.represented_pet_created_by_spell_like_cpp = created_by_spell;
        fixtures.pets.represented_pet_react_state_like_cpp = react_state;
        fixtures.pets.represented_pet_command_state_like_cpp = command_state;
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = canonical;
    if pet_guid.is_none() {
        let _ = host
            .hub_mut()
            .update_player_pet_lifecycle_state_like_cpp(|state| {
                state.temporary_unsummoned_pet_number = 0;
                state.old_pet_spell = 0;
            });
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(fixtures) = host.test_fixtures_like_cpp() {
            fixtures.pets.represented_pet_movement_speed_rates_like_cpp =
                [1.0; UnitMoveTypeLikeCpp::COUNT];
        }
    }
    let _ = host
        .hub_mut()
        .update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = None;
        });
}

pub fn load_represented_pet_spell_rows_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetSpellRowLikeCpp>,
) -> usize {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    let spells: Vec<_> = rows.into_iter().filter(|row| row.spell_id != 0).collect();
    let loaded = spells.len();
    if loaded == 0 {
        lifecycle.pet_load_remove_spells_for_pet_number_like_cpp(pet_number);
    } else {
        lifecycle.pet_load_insert_spells_for_pet_number_like_cpp(pet_number, spells);
    }
    loaded
}

pub fn load_represented_pet_spell_cooldown_rows_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetSpellCooldownRowLikeCpp>,
) -> usize {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    let spell_store = hub.catalogs.spell_store().cloned();
    let cooldowns: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            row.spell_id != 0
                && spell_store
                    .as_ref()
                    .is_none_or(|store| store.get(row.spell_id as i32).is_some())
        })
        .collect();
    let loaded = cooldowns.len();
    if loaded == 0 {
        lifecycle.pet_load_remove_spell_cooldowns_for_pet_number_like_cpp(pet_number);
    } else {
        lifecycle.pet_load_insert_spell_cooldowns_for_pet_number_like_cpp(pet_number, cooldowns);
    }
    loaded
}

pub fn load_represented_pet_spell_charge_rows_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetSpellChargeRowLikeCpp>,
) -> usize {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    let spell_category_store = hub.catalogs.spell_catalogs.spell_category_store().cloned();
    let charges: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            row.category_id != 0
                && spell_category_store
                    .as_ref()
                    .is_none_or(|store| store.get(row.category_id).is_some())
        })
        .collect();
    let loaded = charges.len();
    if loaded == 0 {
        lifecycle.pet_load_remove_spell_charges_for_pet_number_like_cpp(pet_number);
    } else {
        lifecycle.pet_load_insert_spell_charges_for_pet_number_like_cpp(pet_number, charges);
    }
    loaded
}

pub fn load_represented_pet_aura_rows_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
) -> usize {
    load_represented_pet_aura_rows_with_timediff_like_cpp(host, pet_number, rows, 0)
}

pub fn load_represented_pet_aura_rows_with_timediff_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
    timediff_secs: u32,
) -> usize {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    let spell_store = hub.catalogs.spell_store().cloned();
    let difficulty_store = hub.catalogs.difficulty_store().cloned();
    let aura_options_store = hub.catalogs.spell_catalogs.spell_aura_options_store.clone();
    let spell_misc_store = hub.catalogs.spell_catalogs.spell_misc_store().cloned();
    let auras: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            row.spell_id != 0
                && spell_store
                    .as_ref()
                    .is_none_or(|store| store.get(row.spell_id as i32).is_some())
                && (row.difficulty == 0
                    || difficulty_store
                        .as_ref()
                        .is_none_or(|store| store.contains(u32::from(row.difficulty))))
        })
        .filter_map(|mut row| {
            let aura_expires_offline = spell_misc_store
                .as_ref()
                .and_then(|store| store.get_by_spell_id(row.spell_id))
                .is_some_and(|misc| {
                    (misc.attributes[4] as u32
                        & wow_data::spell::attributes::SPELL_ATTR4_AURA_EXPIRES_OFFLINE)
                        != 0
                });
            if let Some(remain_time_ms) = adjusted_represented_pet_aura_remain_time_like_cpp(
                row.remain_time_ms,
                timediff_secs,
                true,
                aura_expires_offline,
            ) {
                row.remain_time_ms = remain_time_ms;
            } else {
                return None;
            }

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
            Some(row)
        })
        .collect();
    let loaded = auras.len();
    if loaded == 0 {
        lifecycle.pet_load_remove_auras_for_pet_number_like_cpp(pet_number);
    } else {
        lifecycle.pet_load_insert_auras_for_pet_number_like_cpp(pet_number, auras);
    }
    loaded
}

pub fn load_represented_pet_aura_effect_rows_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    rows: impl IntoIterator<Item = CharacterPetAuraEffectRowLikeCpp>,
) -> usize {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    let effects: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            row.spell_id != 0
                && u32::from(row.effect_index) < wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP
        })
        .collect();
    let loaded = effects.len();
    if loaded == 0 {
        lifecycle.pet_load_remove_aura_effects_for_pet_number_like_cpp(pet_number);
    } else {
        lifecycle.pet_load_insert_aura_effects_for_pet_number_like_cpp(pet_number, effects);
    }
    loaded
}

pub fn begin_represented_character_pet_authority_load_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
) {
    let (lifecycle, mut hub) = host.split_pets_mut();
    lifecycle.pet_load_reset_like_cpp();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
}

pub fn load_represented_pet_declined_names_like_cpp<H: PetSessionHostLikeCpp>(
    host: &mut H,
    pet_number: u32,
    row: Option<CharacterPetDeclinedNamesRowLikeCpp>,
) -> bool {
    let (lifecycle, mut hub) = host.split_pets_mut();
    hub.invalidate_represented_character_pet_empty_authority_like_cpp();
    if let Some(row) = row {
        lifecycle.pet_load_insert_declined_names_for_pet_number_like_cpp(pet_number, row);
        true
    } else {
        lifecycle
            .pet_load_remove_declined_names_for_pet_number_like_cpp(pet_number)
            .is_some()
    }
}
