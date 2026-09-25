// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet dismissal: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{Arc, ObjectGuid, WorldSession};

impl WorldSession {
    pub(crate) fn destroy_represented_totem_like_cpp(
        &mut self,
        client_slot: u8,
        requested_totem_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if self.player_moved_unit_guid_like_cpp() != Some(player_guid) {
            return false;
        }

        let slot_id = usize::from(client_slot).saturating_add(wow_entities::UNIT_SUMMON_SLOT_TOTEM);
        if slot_id >= wow_entities::MAX_UNIT_TOTEM_SLOT {
            return false;
        }

        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let Some(managed) = manager.find_map_mut(map_id, instance_id.unwrap_or(0)) else {
            return false;
        };
        let map = managed.map_mut();
        let Some(slot_totem_guid) = map
            .get_typed_player(player_guid)
            .map(|player| player.unit().subsystems().control.summon_slots[slot_id])
        else {
            return false;
        };
        if slot_totem_guid.is_empty() {
            return false;
        }

        let matches_cpp_totem = map
            .with_creature_like_cpp(slot_totem_guid, |totem| {
                totem.is_totem_unit_type_like_cpp()
                    && (requested_totem_guid.is_empty() || totem.guid() == requested_totem_guid)
            })
            .unwrap_or(false);
        if !matches_cpp_totem {
            return false;
        }

        let destroyed = match map.remove_from_map_like_cpp(slot_totem_guid, true) {
            Ok(_) => true,
            Err(wow_map::RemoveFromMapError::ObjectNotFound { .. }) => false,
            Err(_) => false,
        };
        if !destroyed {
            return false;
        }

        if let Some(player) = map.get_typed_player_mut(player_guid) {
            let _ = player
                .unit_mut()
                .subsystems_mut()
                .control
                .clear_summon_slot(slot_id);
        }
        drop(manager);
        true
    }

    pub(crate) fn cancel_represented_pet_aura_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        spell_id: u32,
    ) -> bool {
        if self
            .spell_store()
            .and_then(|store| store.get(spell_id as i32))
            .is_none()
        {
            return false;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let Some(managed) = manager.find_map_mut(map_id, instance_id.unwrap_or(0)) else {
            return false;
        };
        let map = managed.map_mut();
        let owned_or_charmed = map.get_typed_player(player_guid).is_some_and(|player| {
            let control = &player.unit().subsystems().control;
            control.pet_guid() == pet_guid || control.charmed_guid == Some(pet_guid)
        });
        if !owned_or_charmed {
            return false;
        }

        if let Some(pet) = map.get_typed_pet_mut(pet_guid) {
            if !pet.creature().is_alive() {
                drop(manager);
                self.send_packet(&wow_packet::packets::pet::PetActionFeedback {
                    spell_id: 0,
                    response: wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP,
                });
                return false;
            }
            let removed = !pet
                .creature_mut()
                .unit_mut()
                .subsystems_mut()
                .auras
                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                .is_empty();
            return removed;
        }

        if let Some(creature) = map.get_typed_creature_mut(pet_guid) {
            if !creature.is_alive() {
                drop(manager);
                self.send_packet(&wow_packet::packets::pet::PetActionFeedback {
                    spell_id: 0,
                    response: wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP,
                });
                return false;
            }
            let removed = !creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .remove_auras_due_to_spell_like_cpp(spell_id, ObjectGuid::EMPTY, 0)
                .is_empty();
            return removed;
        }

        false
    }
}
