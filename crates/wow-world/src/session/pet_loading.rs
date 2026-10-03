// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet loading: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Item, ObjectGuid, UnitMoveTypeLikeCpp, WorldSession, react_state_from_db_like_cpp};

pub(crate) use wow_world_lifecycle::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
};

pub(crate) use wow_world_core::session::CharacterPetStableRowLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration_ms: i32,
    pub remain_time_ms: i32,
    pub remain_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharacterAuraEffectRowLikeCpp {
    pub caster_guid: ObjectGuid,
    pub spell_id: u32,
    pub effect_mask: u32,
    pub effect_index: u8,
    pub amount: i32,
    pub base_amount: i32,
}

pub(crate) fn adjusted_represented_pet_aura_remain_time_like_cpp(
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

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn set_represented_pet_mode_state_with_spell_like_cpp(
        &mut self,
        pet_guid: Option<ObjectGuid>,
        react_state: u8,
        command_state: u8,
        created_by_spell: u32,
    ) {
        if !crate::session::hub_mut(self).set_player_pet_guid_like_cpp(pet_guid) {
            return;
        }
        crate::session::hub_mut(self)
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let canonical = pet_guid.is_some_and(|pet_guid| {
            self.core
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
        #[cfg(test)]
        if !canonical {
            self.fixtures.pets.represented_pet_created_by_spell_like_cpp = created_by_spell;
            self.fixtures.pets.represented_pet_react_state_like_cpp = react_state;
            self.fixtures.pets.represented_pet_command_state_like_cpp = command_state;
        }
        #[cfg(not(test))]
        let _ = canonical;
        if pet_guid.is_none() {
            let _ =
                crate::session::hub_mut(self).update_player_pet_lifecycle_state_like_cpp(|state| {
                    state.temporary_unsummoned_pet_number = 0;
                    state.old_pet_spell = 0;
                });
            #[cfg(test)]
            {
                self.fixtures
                    .pets
                    .represented_pet_movement_speed_rates_like_cpp =
                    [1.0; UnitMoveTypeLikeCpp::COUNT];
            }
        }
        let _ = crate::session::hub_mut(self).update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = None;
        });
    }
}

impl crate::session::PetsCx<'_> {
    pub(crate) fn load_represented_pet_spell_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellRowLikeCpp>,
    ) -> usize {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let spells: Vec<_> = rows
            .into_iter()
            .filter(|row| row.spell_id != 0)
            .collect();
        let loaded = spells.len();
        if loaded == 0 {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .remove_spells_for_pet_number(pet_number);
        } else {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .insert_spells_for_pet_number(pet_number, spells);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_spell_cooldown_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellCooldownRowLikeCpp>,
    ) -> usize {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_store = self.hub.catalogs.spell_store().cloned();
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
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .remove_spell_cooldowns_for_pet_number(pet_number);
        } else {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .insert_spell_cooldowns_for_pet_number(pet_number, cooldowns);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_spell_charge_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetSpellChargeRowLikeCpp>,
    ) -> usize {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_category_store = self
            .hub
            .catalogs
            .spell_catalogs
            .spell_category_store()
            .cloned();
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
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .remove_spell_charges_for_pet_number(pet_number);
        } else {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .insert_spell_charges_for_pet_number(pet_number, charges);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_aura_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
    ) -> usize {
        self.load_represented_pet_aura_rows_with_timediff_like_cpp(pet_number, rows, 0)
    }

    pub(crate) fn load_represented_pet_aura_rows_with_timediff_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let spell_store = self.hub.catalogs.spell_store().cloned();
        let difficulty_store = self.hub.catalogs.difficulty_store().cloned();
        let aura_options_store = self
            .hub
            .catalogs
            .spell_catalogs
            .spell_aura_options_store
            .clone();
        let spell_misc_store = self.hub.catalogs.spell_catalogs.spell_misc_store().cloned();
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
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .remove_auras_for_pet_number(pet_number);
        } else {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .insert_auras_for_pet_number(pet_number, auras);
        }
        loaded
    }

    pub(crate) fn load_represented_pet_aura_effect_rows_like_cpp(
        &mut self,
        pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetAuraEffectRowLikeCpp>,
    ) -> usize {
        self.hub
            .invalidate_represented_character_pet_empty_authority_like_cpp();
        let effects: Vec<_> = rows
            .into_iter()
            .filter(|row| {
                row.spell_id != 0
                    && u32::from(row.effect_index)
                        < wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP
            })
            .collect();
        let loaded = effects.len();
        if loaded == 0 {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .remove_aura_effects_for_pet_number(pet_number);
        } else {
            self.lifecycle
                .pet_load_query_holder_rows_like_cpp
                .insert_aura_effects_for_pet_number(pet_number, effects);
        }
        loaded
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/pet_loading/f3_shims.rs"]
mod f3_shims;
