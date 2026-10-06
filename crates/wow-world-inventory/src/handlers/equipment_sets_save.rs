// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::SpellItemEnchantmentFlags;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid};
use wow_data::{ItemModifiedAppearanceStore, SpellItemEnchantmentStore};
use wow_entities::{
    INVENTORY_SLOT_BAG_0, PLAYER_SLOT_END,
    PlayerEquipmentSetTypeLikeCpp as RepresentedEquipmentSetTypeLikeCpp,
    PlayerEquipmentSetUpdateStateLikeCpp as RepresentedEquipmentSetUpdateStateLikeCpp,
    PlayerInventoryItem as InventoryItem,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{
    OwnedCollectionsAccessLikeCpp, OwnedEquipmentSetsAccessLikeCpp, OwnedInventoryAccessLikeCpp,
    PacketPublicationAccessLikeCpp,
};

use crate::{
    InventoryState, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP, RepresentedEquipmentSetSavedLikeCpp,
    represented_equipment_set_from_packet_like_cpp,
};

/// Borrowed inputs for one equipment-set save operation.
pub struct EquipmentSetsSaveCxLikeCpp<'a> {
    state: &'a mut InventoryState,
    equipment_sets: OwnedEquipmentSetsAccessLikeCpp<'a>,
    inventory: OwnedInventoryAccessLikeCpp<'a>,
    collections: OwnedCollectionsAccessLikeCpp<'a>,
    item_modified_appearance_store: Option<&'a ItemModifiedAppearanceStore>,
    spell_item_enchantment_store: Option<&'a SpellItemEnchantmentStore>,
    generator: &'a EquipmentSetGuidGeneratorLikeCpp,
    publication: PacketPublicationAccessLikeCpp<'a>,
}

impl<'a> EquipmentSetsSaveCxLikeCpp<'a> {
    pub fn new(
        state: &'a mut InventoryState,
        equipment_sets: OwnedEquipmentSetsAccessLikeCpp<'a>,
        inventory: OwnedInventoryAccessLikeCpp<'a>,
        collections: OwnedCollectionsAccessLikeCpp<'a>,
        item_modified_appearance_store: Option<&'a ItemModifiedAppearanceStore>,
        spell_item_enchantment_store: Option<&'a SpellItemEnchantmentStore>,
        generator: &'a EquipmentSetGuidGeneratorLikeCpp,
        publication: PacketPublicationAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            state,
            equipment_sets,
            inventory,
            collections,
            item_modified_appearance_store,
            spell_item_enchantment_store,
            generator,
            publication,
        }
    }

    fn with_equipment_sets_like_cpp<R>(
        &self,
        mut f: impl FnMut(&wow_entities::PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = self
            .equipment_sets
            .with_equipment_sets_like_cpp(|sets| f(sets));
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.equipment_sets.owner_handle_absent_like_cpp() {
            return Some(f(&self.state.represented_equipment_sets_like_cpp));
        }
        None
    }

    fn with_equipment_sets_mut_like_cpp<R>(
        &mut self,
        mut f: impl FnMut(&mut wow_entities::PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = self
            .equipment_sets
            .with_equipment_sets_mut_like_cpp(|sets| f(sets));
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.equipment_sets.owner_handle_absent_like_cpp() {
            return Some(f(&mut self.state.represented_equipment_sets_like_cpp));
        }
        None
    }

    fn get_inventory_item_by_pos_like_cpp(&self, bag: u8, slot: u8) -> Option<InventoryItem> {
        if bag != INVENTORY_SLOT_BAG_0
            || (slot as usize) >= PLAYER_SLOT_END
            || wow_entities::is_buyback_slot(slot)
        {
            return None;
        }
        self.state
            .resolved_player_inventory_runtime_with_access_like_cpp(&self.inventory)?
            .inventory_items()
            .get(&slot)
            .cloned()
    }

    pub(crate) fn save(
        &mut self,
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
                        let item =
                            self.get_inventory_item_by_pos_like_cpp(INVENTORY_SLOT_BAG_0, i as u8)?;
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
                        if self
                            .item_modified_appearance_store
                            .is_some_and(|store| store.get(appearance_id).is_none())
                        {
                            return None;
                        }

                        if !self
                            .state
                            .has_item_appearance_with_collections_access_like_cpp(
                                &self.collections,
                                appearance_id,
                            )
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
                if self.spell_item_enchantment_store.is_some_and(|store| {
                    store.get(enchant_id).is_none_or(|illusion| {
                        illusion.item_visual == 0
                            || !illusion
                                .flags
                                .contains(SpellItemEnchantmentFlags::ALLOW_TRANSMOG)
                    })
                }) {
                    return None;
                }
            }
        }

        let existing_state =
            self.with_equipment_sets_like_cpp(|sets| sets.set_state_like_cpp(set.guid))?;
        if set.guid != 0 && existing_state.is_none() {
            return None;
        }

        let generated_new_guid = set.guid == 0;
        let guid = if generated_new_guid {
            self.generator.generate()
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
        self.with_equipment_sets_mut_like_cpp(|sets| {
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

    pub fn handle_save_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match wow_packet::packets::misc::SaveEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!("Bad SaveEquipmentSet: {error}");
                return;
            }
        };

        let Some(saved) = self.save(request.set) else {
            return;
        };

        if saved.generated_new_guid {
            self.publication
                .send_packet(&wow_packet::packets::misc::EquipmentSetId {
                    guid: saved.guid,
                    set_type: saved.raw_set_type,
                    set_id: saved.set_id,
                });
        }
    }
}
