// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use crate::{
    MAX_EQUIPMENT_SET_INDEX_LIKE_CPP, RepresentedEquipmentSetSavedLikeCpp,
};
use wow_constants::SpellItemEnchantmentFlags;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, PlayerEquipmentSetLikeCpp as RepresentedEquipmentSetLikeCpp,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef};

pub fn represented_equipment_set_from_packet_like_cpp(
    set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    guid: u64,
    state: wow_entities::PlayerEquipmentSetUpdateStateLikeCpp,
) -> Option<wow_entities::PlayerEquipmentSetLikeCpp> {
    let set_type =
        wow_entities::PlayerEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;
    Some(wow_entities::PlayerEquipmentSetLikeCpp {
        raw_set_type: set.set_type,
        set_type,
        guid,
        set_id: set.set_id,
        ignore_mask: set.ignore_mask,
        pieces: set.pieces,
        appearances: set.appearances,
        enchants: set.enchants,
        secondary_shoulder_appearance_id: set.secondary_shoulder_appearance_id,
        secondary_shoulder_slot: set.secondary_shoulder_slot,
        secondary_weapon_appearance_id: set.secondary_weapon_appearance_id,
        secondary_weapon_slot: set.secondary_weapon_slot,
        assigned_spec_index: set.assigned_spec_index,
        set_name: set.set_name,
        set_icon: set.set_icon,
        state,
    })
}

impl crate::InventoryState {
    pub fn save_represented_equipment_set_with_generator_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        generator: &EquipmentSetGuidGeneratorLikeCpp,
        mut set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    ) -> Option<RepresentedEquipmentSetSavedLikeCpp> {
        if set.set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return None;
        }

        let set_type =
            RepresentedEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;

        for i in 0..wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP {
            let slot_bit = 1_u32 << i;
            if (set.ignore_mask & slot_bit) == 0 {
                if set_type == RepresentedEquipmentSetTypeLikeCpp::Equipment {
                    set.appearances[i] = 0;

                    let item_guid = set.pieces[i];
                    if !item_guid.is_empty() {
                        let item = self.get_inventory_item_by_pos(
                            hub.shared(),
                            INVENTORY_SLOT_BAG_0,
                            i as u8,
                        )?;
                        if item.guid != item_guid {
                            return None;
                        }
                    } else {
                        set.ignore_mask |= slot_bit;
                    }
                } else {
                    set.pieces[i] = ObjectGuid::EMPTY;
                    if set.appearances[i] != 0 {
                        let appearance_id = u32::try_from(set.appearances[i]).ok()?;
                        if hub
                            .catalogs
                            .item_modified_appearance_store()
                            .is_some_and(|store| store.get(appearance_id).is_none())
                        {
                            return None;
                        }

                        if !self
                            .has_item_appearance_like_cpp(hub.shared(), appearance_id)
                            .0
                        {
                            return None;
                        }
                    } else {
                        set.ignore_mask |= slot_bit;
                    }
                }
            } else {
                set.pieces[i] = ObjectGuid::EMPTY;
                set.appearances[i] = 0;
            }
        }

        set.ignore_mask &= 0x7_FFFF;
        if set_type == RepresentedEquipmentSetTypeLikeCpp::Equipment {
            set.enchants = [0, 0];
        } else {
            for enchant_id in set.enchants {
                if enchant_id == 0 {
                    continue;
                }

                let enchant_id = u32::try_from(enchant_id).ok()?;
                if hub
                    .catalogs
                    .spell_item_enchantment_store()
                    .is_some_and(|store| {
                        store.get(enchant_id).is_none_or(|illusion| {
                            illusion.item_visual == 0
                                || !illusion
                                    .flags
                                    .contains(SpellItemEnchantmentFlags::ALLOW_TRANSMOG)
                        })
                    })
                {
                    return None;
                }
            }
        }

        let existing_state = self.with_owned_equipment_sets_like_cpp(hub.shared(), |sets| {
            sets.set_state_like_cpp(set.guid)
        })?;
        if set.guid != 0 && existing_state.is_none() {
            return None;
        }

        let generated_new_guid = set.guid == 0;
        let guid = if generated_new_guid {
            generator.generate()
        } else {
            set.guid
        };
        set.guid = guid;

        // The owner applies C++ `Player::SetEquipmentSet`'s state rule
        // (Player.cpp:26406), so the state passed here is only the row's
        // starting point.
        let saved = represented_equipment_set_from_packet_like_cpp(
            set,
            guid,
            existing_state.unwrap_or(RepresentedEquipmentSetUpdateStateLikeCpp::New),
        )?;
        let saved_raw_type = saved.raw_set_type;
        let saved_set_id = saved.set_id;
        self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| {
            if generated_new_guid {
                sets.create_set_like_cpp(saved.clone());
                true
            } else {
                sets.update_set_like_cpp(saved.clone())
            }
        })?
        .then_some(())?;

        Some(RepresentedEquipmentSetSavedLikeCpp {
            guid,
            set_type,
            raw_set_type: saved_raw_type,
            set_id: saved_set_id,
            generated_new_guid,
        })
    }
}

impl crate::InventoryState {
    pub fn with_owned_equipment_sets_like_cpp<R>(
        &self,
        hub: HubRef<'_>,
        mut f: impl FnMut(&wow_entities::PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(|player| f(&player.gameplay_state().equipment_sets));
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(f(&self.represented_equipment_sets_like_cpp));
        }
        None
    }

    pub fn with_owned_equipment_sets_mut_like_cpp<R>(
        &mut self,
        hub: &mut HubMut<'_>,
        mut f: impl FnMut(&mut wow_entities::PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            f(&mut player.gameplay_state_mut().equipment_sets)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(f(&mut self.represented_equipment_sets_like_cpp));
        }
        None
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn insert_represented_equipment_set_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: u64,
        equipment_set: RepresentedEquipmentSetLikeCpp,
    ) {
        let mut equipment_set = equipment_set;
        equipment_set.guid = guid;
        let _ = self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| {
            sets.install_loaded_set_like_cpp(equipment_set.clone());
        });
    }

    pub fn clear_represented_equipment_sets_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _ = self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| sets.clear_like_cpp());
    }

    pub fn mark_represented_equipment_sets_loaded_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _ =
            self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| sets.mark_loaded_like_cpp());
    }

    pub fn load_represented_equipment_set_row_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: u64,
        set_id: u32,
        set_name: String,
        set_icon: String,
        ignore_mask: u32,
        assigned_spec_index: i32,
        pieces: [ObjectGuid; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
    ) -> bool {
        if set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return false;
        }

        let equipment_set = RepresentedEquipmentSetLikeCpp {
            raw_set_type: RepresentedEquipmentSetTypeLikeCpp::Equipment.as_i32_like_cpp(),
            set_type: RepresentedEquipmentSetTypeLikeCpp::Equipment,
            guid,
            set_id,
            ignore_mask,
            pieces,
            appearances: [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP],
            enchants: [0; 2],
            secondary_shoulder_appearance_id: 0,
            secondary_shoulder_slot: 0,
            secondary_weapon_appearance_id: 0,
            secondary_weapon_slot: 0,
            assigned_spec_index,
            set_name,
            set_icon,
            state: RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged,
        };
        self.with_owned_equipment_sets_mut_like_cpp(hub, |sets| {
            sets.install_loaded_set_like_cpp(equipment_set.clone());
        })
        .is_some()
    }

    pub fn represented_load_equipment_set_packet_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<wow_packet::packets::misc::LoadEquipmentSet> {
        self.with_owned_equipment_sets_like_cpp(hub, |stored| {
            let sets = stored
                .sets_like_cpp()
                .values()
                .filter(|equipment_set| {
                    equipment_set.state != RepresentedEquipmentSetUpdateStateLikeCpp::Deleted
                })
                .map(
                    |equipment_set| wow_packet::packets::misc::EquipmentSetDataLikeCpp {
                        set_type: equipment_set.raw_set_type,
                        guid: equipment_set.guid,
                        set_id: equipment_set.set_id,
                        ignore_mask: equipment_set.ignore_mask,
                        pieces: equipment_set.pieces,
                        appearances: equipment_set.appearances,
                        enchants: equipment_set.enchants,
                        secondary_shoulder_appearance_id: equipment_set
                            .secondary_shoulder_appearance_id,
                        secondary_shoulder_slot: equipment_set.secondary_shoulder_slot,
                        secondary_weapon_appearance_id: equipment_set
                            .secondary_weapon_appearance_id,
                        secondary_weapon_slot: equipment_set.secondary_weapon_slot,
                        assigned_spec_index: equipment_set.assigned_spec_index,
                        set_name: equipment_set.set_name.clone(),
                        set_icon: equipment_set.set_icon.clone(),
                    },
                )
                .collect();
            wow_packet::packets::misc::LoadEquipmentSet { sets }
        })
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_equipment_set_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: u64,
    ) -> Option<RepresentedEquipmentSetLikeCpp> {
        self.with_owned_equipment_sets_like_cpp(hub, |sets| sets.set_like_cpp(guid).cloned())?
    }

    pub fn assign_represented_equipment_set_to_spec_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        set_id: u32,
        spec_index: u32,
    ) -> bool {
        let owner = hub.core.owned_equipment_sets_access_like_cpp();
        crate::handlers::EquipmentSetsHandlerCxLikeCpp::new(self, owner)
            .assign_represented_equipment_set_to_spec_like_cpp(set_id, spec_index)
    }

    pub fn delete_represented_equipment_set_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        id: u64,
    ) -> bool {
        let owner = hub.core.owned_equipment_sets_access_like_cpp();
        crate::handlers::EquipmentSetsHandlerCxLikeCpp::new(self, owner)
            .delete_represented_equipment_set_like_cpp(id)
    }

    /// Install the process-wide C++ `sObjectMgr->GenerateEquipmentSetGuid()`
    /// mirror shared by equipment sets and transmog outfits for every player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn set_equipment_set_guid_generator_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        generator: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    ) {
        hub.core.equipment_set_guid_generator_like_cpp = Some(generator);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn equipment_set_guid_generator_for_test_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Arc<EquipmentSetGuidGeneratorLikeCpp>> {
        hub.core.equipment_set_guid_generator_like_cpp.clone()
    }

    /// Test-fixture acknowledgement of C++ `Player::_SaveEquipmentSets`
    /// (`Player.cpp:26409-26500`): after persistence statements are queued,
    /// surviving rows become unchanged and deleted rows are removed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mark_equipment_sets_saved_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) {
        let _ = self
            .with_owned_equipment_sets_mut_like_cpp(hub, |sets| sets.mark_sets_saved_like_cpp());
    }
}
