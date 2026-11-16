// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Equipment slots, transmogrification and durability.

use super::super::*;
use crate::player_rules::set_dynamic_update_mask_index;

impl Player {
    /// C++ `Unit::haveOffhandWeapon`/`Player::GetWeaponForAttack` admission.
    ///
    /// Player dual-wield capability is only one prerequisite. TrinityCore
    /// resolves the concrete offhand item, requires an item-class weapon and,
    /// for the `useable` query used by `haveOffhandWeapon`, rejects a broken
    /// item (`Unit.cpp:496`, `Player.cpp:9243-9270`). The canonical Player
    /// inventory runtime is the sole Rust owner of both the slot record and
    /// item object, so this check deliberately does not infer an equipped
    /// weapon from `can_dual_wield` or from the cached damage bounds alone.
    pub fn has_offhand_weapon_for_attack_like_cpp(&self) -> bool {
        let Some(slot_item) = self
            .inventory_runtime_like_cpp()
            .inventory_items()
            .get(&EQUIPMENT_SLOT_OFFHAND)
        else {
            return false;
        };

        let Some(inventory_type) = slot_item.inventory_type else {
            return false;
        };
        let is_weapon_inventory_type = matches!(
            inventory_type,
            value if value == InventoryType::Weapon as u8
                || value == InventoryType::WeaponMainhand as u8
                || value == InventoryType::WeaponOffhand as u8
                || value == InventoryType::Weapon2Hand as u8
                || value == InventoryType::Ranged as u8
                || value == InventoryType::RangedRight as u8
        );
        if !is_weapon_inventory_type {
            return false;
        }

        self.inventory_runtime_like_cpp()
            .item_objects()
            .get(&slot_item.guid)
            .is_some_and(|item| !item.is_broken())
    }

    /// C++ `Unit::IsInFeralForm` (`Unit.cpp:8807-8812`) for the Player-owned
    /// shapeshift projection. The unit update field is authoritative when it
    /// is present; the transitional Player gameplay projection is retained as
    /// a fallback until aura state owns shapeshift changes for every runtime
    /// path.
    pub fn is_in_feral_form_like_cpp(&self) -> bool {
        matches!(
            self.unit().shapeshift_form_like_cpp(),
            wow_constants::ShapeShiftForm::CatForm
                | wow_constants::ShapeShiftForm::BearForm
                | wow_constants::ShapeShiftForm::DireBearForm
                | wow_constants::ShapeShiftForm::GhostWolf
        ) || matches!(self.shapeshift_form_id_like_cpp(), 1 | 5 | 8 | 16)
    }

    pub fn equip_item_object(
        &mut self,
        pos: u16,
        item: &mut Item,
        existing: Option<&mut Item>,
        visible: VisibleItemValues,
    ) -> Result<EquipItemObjectOutcome, PlayerStorageError> {
        let bag = (pos >> 8) as u8;
        let slot = pos as u8;
        if bag != INVENTORY_SLOT_BAG_0 {
            return Err(PlayerStorageError::UnknownBag(bag));
        }
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        match existing {
            None => {
                if self.top_level_item_guid(slot).is_some() {
                    return Err(PlayerStorageError::OccupiedPlayerSlot(slot));
                }

                self.visualize_item_object(slot, item, visible)?;
                item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
                Ok(EquipItemObjectOutcome::Equipped)
            }
            Some(existing) => {
                let Some(expected_guid) = self.top_level_item_guid(slot) else {
                    return Err(PlayerStorageError::EmptyPlayerSlot(slot));
                };

                let actual_guid = existing.object().guid();
                if expected_guid != actual_guid {
                    return Err(PlayerStorageError::MismatchedItemGuid {
                        slot,
                        expected: expected_guid,
                        actual: actual_guid,
                    });
                }

                existing.set_count(existing.count() + item.count());
                existing.set_state(ItemUpdateState::Changed);

                item.set_owner_guid(self.guid());
                item.set_not_refundable();
                item.clear_soulbound_tradeable();
                item.set_state(ItemUpdateState::Removed);
                Ok(EquipItemObjectOutcome::Merged)
            }
        }
    }

    pub fn quick_equip_item_object(
        &mut self,
        pos: u16,
        item: &mut Item,
        visible: VisibleItemValues,
    ) -> Result<(), PlayerStorageError> {
        let bag = (pos >> 8) as u8;
        let slot = pos as u8;
        if bag != INVENTORY_SLOT_BAG_0 {
            return Err(PlayerStorageError::UnknownBag(bag));
        }
        if slot as usize >= PLAYER_SLOT_END {
            return Err(PlayerStorageError::InvalidPlayerSlot(slot));
        }

        self.visualize_item_object(slot, item, visible)?;
        item.set_item_flag2(ItemFieldFlags2::EQUIPPED);
        Ok(())
    }

    /// C++ `Player::AddTransmogBlock`.
    pub fn add_transmog_block_like_cpp(&mut self, block_value: u32) -> usize {
        let index = self.active_data.transmog.len();
        self.active_data.transmog.push(block_value);
        set_dynamic_update_mask_index(&mut self.active_data.transmog_update_mask, index);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_TRANSMOG_BIT);
        index
    }

    /// C++ `Player::AddTransmogFlag`.
    pub fn add_transmog_flag_like_cpp(&mut self, slot: usize, flag: u32) -> bool {
        let Some(block) = self.active_data.transmog.get_mut(slot) else {
            return false;
        };
        let new_block = *block | flag;
        if new_block == *block {
            return false;
        }

        *block = new_block;
        set_dynamic_update_mask_index(&mut self.active_data.transmog_update_mask, slot);
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_TRANSMOG_BIT);
        true
    }

    pub fn transmog_blocks_like_cpp(&self) -> &[u32] {
        &self.active_data.transmog
    }

    /// C++ `Player::AddConditionalTransmog`.
    pub fn add_conditional_transmog_like_cpp(&mut self, item_modified_appearance_id: u32) -> usize {
        let index = self.active_data.conditional_transmog.len();
        self.active_data
            .conditional_transmog
            .push(item_modified_appearance_id as i32);
        set_dynamic_update_mask_index(
            &mut self.active_data.conditional_transmog_update_mask,
            index,
        );
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT);
        index
    }

    /// C++ `Player::RemoveConditionalTransmog`.
    pub fn remove_conditional_transmog_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> bool {
        let Some(index) = self
            .active_data
            .conditional_transmog
            .iter()
            .position(|id| *id == item_modified_appearance_id as i32)
        else {
            return false;
        };

        self.active_data.conditional_transmog.remove(index);
        set_dynamic_update_mask_index(
            &mut self.active_data.conditional_transmog_update_mask,
            index,
        );
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT);
        true
    }

    pub fn conditional_transmog_like_cpp(&self) -> &[i32] {
        &self.active_data.conditional_transmog
    }
}
