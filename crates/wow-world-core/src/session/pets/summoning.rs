//! Pet summoning, dismissal and stable operations.

use std::sync::Arc;

use crate::session::movement_protocol::{
    TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP, TeleportToOptionsLikeCpp,
};
use crate::session::{
    CharacterPetStableRowLikeCpp, pet_type_from_db_like_cpp, react_state_from_db_like_cpp,
};
use wow_constants::movement::MovementFlag;
use wow_entities::{Pet, PetSaveMode, PetStable, PetStableInfo};

impl crate::session::HubRef<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_pet_stable_current_index_like_cpp(&self) -> Option<u32> {
        self.player_pet_lifecycle_state_snapshot_like_cpp()
            .and_then(|state| state.stable.current_pet_index)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_temporary_unsummoned_pet_number_like_cpp(&self) -> u32 {
        self.player_pet_lifecycle_state_snapshot_like_cpp()
            .map_or(0, |state| state.temporary_unsummoned_pet_number)
    }

    pub fn represented_pet_stable_info_by_number_like_cpp(
        &self,
        pet_number: u32,
    ) -> Option<PetStableInfo> {
        let pet_lifecycle = self.player_pet_lifecycle_state_snapshot_like_cpp()?;
        pet_lifecycle
            .stable
            .active_pets
            .iter()
            .chain(pet_lifecycle.stable.stabled_pets.iter())
            .flatten()
            .find(|pet| pet.pet_number == pet_number)
            .cloned()
            .or_else(|| {
                pet_lifecycle
                    .stable
                    .unslotted_pets
                    .iter()
                    .find(|pet| pet.pet_number == pet_number)
                    .cloned()
            })
    }

    pub fn is_pet_need_be_temporary_unsummoned_like_cpp(&self) -> bool {
        !self.core.player_is_in_world_for_registry_like_cpp()
            || self.resolved_player_is_alive_like_cpp() != Some(true)
            || self
                .resolved_player_movement_flags_like_cpp()
                .is_none_or(|flags| flags.contains(MovementFlag::FLYING))
    }
}

impl crate::session::HubMut<'_> {
    pub fn unsummon_represented_pet_for_same_map_teleport_if_out_of_range_like_cpp(
        &mut self,
        destination: wow_core::Position,
        options: TeleportToOptionsLikeCpp,
    ) {
        if options & TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP != 0 {
            return;
        }
        let Some(pet_guid) = self.shared().player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        let map_id = u32::from(self.core.player_map_id_like_cpp());
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        let should_unsummon = {
            let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
                return;
            };
            let Ok(manager) = manager.lock() else {
                return;
            };
            let Some(managed) = manager.find_map(map_id, instance_id) else {
                return;
            };
            let visibility_range = managed.map().visibility_range();
            managed
                .map()
                .get_pet(pet_guid)
                .is_some_and(|pet| pet.distance_to_position(destination) > visibility_range)
        };

        if should_unsummon {
            self.unsummon_represented_pet_temporary_if_any_like_cpp();
        }
    }

    pub fn set_represented_pet_stable_like_cpp(&mut self, stable: PetStable) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| state.stable = stable);
    }

    pub fn load_represented_pet_stable_rows_like_cpp(
        &mut self,
        summoned_pet_number: u32,
        rows: impl IntoIterator<Item = CharacterPetStableRowLikeCpp>,
    ) -> usize {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let mut stable = PetStable::default();
        let mut loaded = 0usize;
        let mut query_had_rows = false;

        for row in rows {
            query_had_rows = true;
            let slot = row.slot;
            let pet_info = PetStableInfo {
                name: row.name,
                action_bar: row.action_bar,
                pet_number: row.pet_number,
                creature_id: row.creature_id,
                display_id: row.display_id,
                experience: row.experience,
                health: row.health,
                mana: row.mana,
                last_save_time: row.last_save_time,
                created_by_spell_id: row.created_by_spell_id,
                specialization_id: row.specialization_id,
                level: row.level,
                react_state: react_state_from_db_like_cpp(row.react_state),
                pet_type: pet_type_from_db_like_cpp(row.pet_type),
                was_renamed: row.was_renamed,
            };

            if PetSaveMode::is_active_slot(slot) {
                let index = slot as usize;
                if stable.active_pets.len() <= index {
                    stable.active_pets.resize_with(index + 1, || None);
                }
                stable.active_pets[index] = Some(pet_info);
            } else if PetSaveMode::is_stabled_slot(slot) {
                let index = (slot - PetSaveMode::stable_slot(0)) as usize;
                if stable.stabled_pets.len() <= index {
                    stable.stabled_pets.resize_with(index + 1, || None);
                }
                stable.stabled_pets[index] = Some(pet_info);
            } else if slot == PetSaveMode::NotInSlot as i16 {
                stable.unslotted_pets.push(pet_info);
            } else {
                continue;
            }

            loaded = loaded.saturating_add(1);
        }

        let selected_summoned_pet =
            Pet::get_load_pet_info(&stable, 0, summoned_pet_number, None).is_some();
        if selected_summoned_pet {
            stable.current_pet_index = stable
                .active_pets
                .iter()
                .position(|pet| {
                    pet.as_ref()
                        .is_some_and(|pet| pet.pet_number == summoned_pet_number)
                })
                .map(|index| index as u32);
        }

        if !self.update_player_pet_lifecycle_state_like_cpp(|state| {
            // C++ remembers the selected controlled pet on the Player even
            // while the live Pet object is absent from Map storage.
            if selected_summoned_pet {
                state.temporary_unsummoned_pet_number = summoned_pet_number;
            }
            state.stable = stable;
            state.character_rows_empty_authority_complete = !query_had_rows;
        }) {
            return 0;
        }
        loaded
    }

    pub fn unsummon_represented_pet_temporary_if_any_like_cpp(&mut self) {
        let Some(pet_guid) = self.shared().player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        let Some(pet_lifecycle) = self.shared().player_pet_lifecycle_state_snapshot_like_cpp()
        else {
            return;
        };

        self.request_temporary_pet_unsummon_like_cpp();

        if let Some(manager) = self.core.canonical_map_manager.as_ref().map(Arc::clone)
            && let Ok(mut manager) = manager.lock()
        {
            let mut removed = false;
            let mut temporary_pet_number = None;
            let mut temporary_pet_created_by_spell = 0;
            manager.do_for_all_maps_mut(|managed| {
                if removed {
                    return;
                }
                let Some(pet) = managed.map().get_typed_pet(pet_guid) else {
                    return;
                };
                if pet_lifecycle.temporary_unsummoned_pet_number == 0
                    && pet.is_controlled()
                    && !pet.is_temporary_summoned()
                {
                    temporary_pet_number = pet
                        .creature()
                        .unit()
                        .subsystems()
                        .control
                        .charm_info
                        .as_ref()
                        .map(|charm_info| charm_info.pet_number);
                    temporary_pet_created_by_spell = pet.created_by_spell_id_like_cpp();
                }
                match managed.map_mut().remove_from_map_like_cpp(pet_guid, false) {
                    Ok(_) => removed = true,
                    Err(wow_map::RemoveFromMapError::ObjectNotFound { .. }) => {}
                    Err(_) => {}
                }
            });
            if let Some(pet_number) = temporary_pet_number.filter(|pet_number| *pet_number != 0) {
                let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
                    state.temporary_unsummoned_pet_number = pet_number;
                    state.old_pet_spell = temporary_pet_created_by_spell;
                });
            }
        }

        let _ = self.set_player_pet_guid_like_cpp(None);
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.pets.represented_pet_created_by_spell_like_cpp = 0;
            self.fixtures.pets.represented_pet_react_state_like_cpp =
                wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP;
            self.fixtures.pets.represented_pet_command_state_like_cpp =
                wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
        }
        let _ = self.update_player_pet_lifecycle_state_like_cpp(|state| {
            state.temporary_mount_react_state = None;
        });
    }
}

impl crate::session::HubMut<'_> {
    pub fn request_temporary_pet_unsummon_like_cpp(&mut self) {
        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.pets.temporary_pet_unsummon_requests_like_cpp = self
                .fixtures
                .pets
                .temporary_pet_unsummon_requests_like_cpp
                .saturating_add(1);
        }
    }
}
