// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Equipment slots, transmogrification and durability.

use super::super::*;

impl Player {
    pub fn find_equip_slot(&self, args: FindEquipSlotArgs<'_>) -> u8 {
        let slots = equip_slot_candidates(args);
        if slots[0] == NULL_SLOT {
            return NULL_SLOT;
        }

        if args.slot != NULL_SLOT {
            if args.swap
                || item_ref_by_pos(args.equipped_items, INVENTORY_SLOT_BAG_0, args.slot).is_none()
            {
                for candidate in slots {
                    if candidate == args.slot {
                        return args.slot;
                    }
                }
            }
        } else {
            for candidate in slots {
                if candidate != NULL_SLOT
                    && item_ref_by_pos(args.equipped_items, INVENTORY_SLOT_BAG_0, candidate)
                        .is_none()
                    && (candidate != EQUIPMENT_SLOT_OFFHAND || !args.is_two_hand_used)
                {
                    return candidate;
                }
            }

            if args.swap {
                let mut min_item_level = u32::MAX;
                let mut min_item_level_index = 0usize;
                for (index, candidate) in slots.into_iter().enumerate() {
                    if candidate == NULL_SLOT {
                        continue;
                    }

                    if let Some(equipped) =
                        item_ref_by_pos(args.equipped_items, INVENTORY_SLOT_BAG_0, candidate)
                    {
                        let item_level = u32::from(equipped.data().debug_item_level);
                        if item_level < min_item_level {
                            min_item_level = item_level;
                            min_item_level_index = index;
                        }
                    }
                }

                return slots[min_item_level_index];
            }
        }

        NULL_SLOT
    }

    pub fn can_equip_item(&self, args: CanEquipItemArgs<'_>) -> CanEquipItemOutcome {
        let Some(source) = args.source_item else {
            return can_equip_item_outcome(if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            });
        };

        let Some(proto) = args.proto else {
            return can_equip_item_outcome(if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            });
        };

        if source.loot_generated() {
            return can_equip_item_outcome(InventoryResult::LootGone);
        }

        if source.is_binded_not_with(self.guid(), proto, args.source_bop_trade_allowed_for_player) {
            return can_equip_item_outcome(InventoryResult::NotOwner);
        }

        let similar_result = self.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: args.proto,
            count: source.count(),
            source_item: args.source_item,
            current_item_count: self.item_count_by_entry(
                proto.entry,
                false,
                args.source_item,
                args.stored_items,
            ),
            limit_category: args.limit_category,
            current_limit_category_count: self.item_count_with_limit_category(
                proto.item_limit_category,
                args.source_item,
                args.stored_items,
            ),
        });
        if similar_result.result != InventoryResult::Ok {
            return can_equip_item_outcome(similar_result.result);
        }

        if args.not_loading {
            if args.is_stunned {
                return can_equip_item_outcome(InventoryResult::GenericStunned);
            }

            if args.is_charmed {
                return can_equip_item_outcome(InventoryResult::ClientLockedOut);
            }

            if !proto.can_change_equip_state_in_combat() {
                if args.is_in_combat {
                    return can_equip_item_outcome(InventoryResult::NotInCombat);
                }

                if args.is_in_progress_arena {
                    return can_equip_item_outcome(InventoryResult::NotDuringArenaMatch);
                }
            }

            if args.is_in_combat
                && (proto.class_id == ItemClass::Weapon
                    || proto.inventory_type == InventoryType::Relic)
                && args.weapon_change_timer_active
            {
                return can_equip_item_outcome(InventoryResult::ItemCooldown);
            }

            if matches!(args.current_generic_spell_allows_equip, Some(false))
                || matches!(args.current_channeled_spell_allows_equip, Some(false))
            {
                return can_equip_item_outcome(InventoryResult::ClientLockedOut);
            }
        }

        if args.heirloom_required_level_failed {
            return can_equip_item_outcome(InventoryResult::NotEquippable);
        }

        let eslot = self.find_equip_slot(FindEquipSlotArgs {
            proto,
            slot: args.slot,
            swap: args.swap,
            can_dual_wield: args.can_dual_wield,
            can_titan_grip: args.can_titan_grip,
            is_two_hand_used: args.is_two_hand_used,
            has_required_profession_skill: args.has_required_profession_skill,
            profession_slot: args.profession_slot,
            equipped_items: args.equipped_items,
        });
        if eslot == NULL_SLOT {
            return can_equip_item_outcome(InventoryResult::NotEquippable);
        }

        if args.can_use_result != InventoryResult::Ok {
            return can_equip_item_outcome(args.can_use_result);
        }

        if !args.swap && item_ref_by_pos(args.equipped_items, INVENTORY_SLOT_BAG_0, eslot).is_some()
        {
            return can_equip_item_outcome(InventoryResult::NoSlotAvailable);
        }

        let mut ignore = paired_unique_ignore_slot(eslot).unwrap_or(NULL_SLOT);
        if ignore == NULL_SLOT
            || !item_ref_by_pos(args.equipped_items, INVENTORY_SLOT_BAG_0, ignore)
                .is_some_and(|equipped| std::ptr::eq(equipped, source))
        {
            ignore = eslot;
        }
        let unique_ignore_slot = if args.swap { ignore } else { NULL_SLOT };
        if args.can_equip_unique_result != InventoryResult::Ok {
            return CanEquipItemOutcome {
                result: args.can_equip_unique_result,
                dest: 0,
                unique_ignore_slot: Some(unique_ignore_slot),
            };
        }

        if proto.class_id == ItemClass::Quiver {
            for stored in args.stored_items {
                if stored.bag != INVENTORY_SLOT_BAG_0
                    || !(INVENTORY_SLOT_BAG_START..INVENTORY_SLOT_BAG_END).contains(&stored.slot)
                    || std::ptr::eq(stored.item, source)
                {
                    continue;
                }

                if let Some(bag_proto) = stored.template {
                    if bag_proto.class_id == proto.class_id && (!args.swap || stored.slot != eslot)
                    {
                        return CanEquipItemOutcome {
                            result: if bag_proto.subclass_id == ItemSubClassQuiver::AmmoPouch as u32
                            {
                                InventoryResult::OnlyOneAmmo
                            } else {
                                InventoryResult::OnlyOneQuiver
                            },
                            dest: 0,
                            unique_ignore_slot: Some(unique_ignore_slot),
                        };
                    }
                }
            }
        }

        if eslot == EQUIPMENT_SLOT_OFFHAND {
            match proto.inventory_type {
                InventoryType::Weapon
                    if proto.subclass_id == ItemSubClassWeapon::Polearm as u32 =>
                {
                    return can_equip_item_outcome(InventoryResult::TwoHandSkillNotFound);
                }
                InventoryType::Weapon if !args.can_dual_wield => {
                    return can_equip_item_outcome(InventoryResult::TwoHandSkillNotFound);
                }
                InventoryType::WeaponOffhand
                    if !args.can_dual_wield && !args.proto_always_allow_dual_wield =>
                {
                    return can_equip_item_outcome(InventoryResult::TwoHandSkillNotFound);
                }
                InventoryType::Weapon2Hand if !args.can_dual_wield || !args.can_titan_grip => {
                    return can_equip_item_outcome(InventoryResult::TwoHandSkillNotFound);
                }
                _ => {}
            }

            if args.is_two_hand_used {
                return can_equip_item_outcome(InventoryResult::Equipped2handed);
            }
        }

        if proto.inventory_type == InventoryType::Weapon2Hand {
            if eslot == EQUIPMENT_SLOT_OFFHAND {
                if !args.can_titan_grip {
                    return can_equip_item_outcome(InventoryResult::NotEquippable);
                }
            } else if eslot != EQUIPMENT_SLOT_MAINHAND {
                return can_equip_item_outcome(InventoryResult::NotEquippable);
            }

            if !args.can_titan_grip
                && item_ref_by_pos(
                    args.equipped_items,
                    INVENTORY_SLOT_BAG_0,
                    EQUIPMENT_SLOT_OFFHAND,
                )
                .is_some()
                && (!args.not_loading
                    || args.offhand_can_unequip_result != InventoryResult::Ok
                    || args.offhand_can_store_result != InventoryResult::Ok)
            {
                return can_equip_item_outcome(if args.swap {
                    InventoryResult::CantSwap
                } else {
                    InventoryResult::InvFull
                });
            }
        }

        CanEquipItemOutcome {
            result: InventoryResult::Ok,
            dest: make_item_pos(INVENTORY_SLOT_BAG_0, eslot),
            unique_ignore_slot: Some(unique_ignore_slot),
        }
    }

    pub fn can_unequip_item(&self, args: CanUnequipItemArgs<'_>) -> InventoryResult {
        if !is_equipment_packed_pos(args.pos) && !is_bag_pos(args.pos) {
            return InventoryResult::Ok;
        }

        let Some(source) = args.source_item else {
            return InventoryResult::Ok;
        };

        let Some(proto) = args.proto else {
            return InventoryResult::ItemNotFound;
        };

        if source.loot_generated() {
            return InventoryResult::LootGone;
        }

        if args.is_charmed {
            return InventoryResult::ClientLockedOut;
        }

        if !proto.can_change_equip_state_in_combat() {
            if args.is_in_combat {
                return InventoryResult::NotInCombat;
            }

            if args.is_in_progress_arena {
                return InventoryResult::NotDuringArenaMatch;
            }
        }

        if !args.swap && args.source_is_not_empty_bag {
            return InventoryResult::DestroyNonemptyBag;
        }

        InventoryResult::Ok
    }

    pub fn can_equip_unique_item_template(
        &self,
        args: CanEquipUniqueItemTemplateArgs<'_>,
    ) -> InventoryResult {
        let Some(proto) = args.proto else {
            return InventoryResult::ItemNotFound;
        };

        if args.unique_equippable
            && (has_equipped_item_entry(args.equipped_items, proto.entry, args.except_slot)
                || has_equipped_gem_entry(args.equipped_gems, proto.entry, args.except_slot))
        {
            return InventoryResult::ItemUniqueEquippable;
        }

        if proto.item_limit_category != 0 {
            let Some(limit_category) = args.limit_category else {
                return InventoryResult::NotEquippable;
            };
            let limit_quantity = u32::from(limit_category.quantity);

            if args.limit_count > limit_quantity {
                return InventoryResult::ItemMaxLimitCategoryEquippedExceededIs;
            }

            let required_count = limit_quantity.saturating_sub(args.limit_count) + 1;
            if equipped_item_limit_category_count(
                args.equipped_items,
                proto.item_limit_category,
                args.except_slot,
            ) >= required_count
            {
                return InventoryResult::ItemMaxLimitCategoryEquippedExceededIs;
            }

            if equipped_gem_limit_category_count(
                args.equipped_gems,
                proto.item_limit_category,
                args.except_slot,
            ) >= required_count
            {
                return InventoryResult::ItemMaxCountEquippedSocketed;
            }
        }

        InventoryResult::Ok
    }

    pub fn can_equip_unique_item(&self, args: CanEquipUniqueItemArgs<'_>) -> InventoryResult {
        let Some(source) = args.source_item else {
            return InventoryResult::ItemNotFound;
        };

        let template_result = self.can_equip_unique_item_template(CanEquipUniqueItemTemplateArgs {
            proto: args.proto,
            except_slot: args.except_slot,
            limit_count: args.limit_count,
            unique_equippable: args.unique_equippable,
            limit_category: args.limit_category,
            equipped_items: args.equipped_items,
            equipped_gems: args.equipped_gems,
        });
        if template_result != InventoryResult::Ok {
            return template_result;
        }

        for gem in args.socketed_gems {
            let Some(gem_proto) = gem.proto else {
                continue;
            };

            let gem_limit_count = if !source.is_equipped() && gem_proto.item_limit_category != 0 {
                gem.source_limit_category_count
            } else {
                1
            };

            let gem_result = self.can_equip_unique_item_template(CanEquipUniqueItemTemplateArgs {
                proto: Some(gem_proto),
                except_slot: args.except_slot,
                limit_count: gem_limit_count,
                unique_equippable: gem.unique_equippable,
                limit_category: gem.limit_category,
                equipped_items: args.equipped_items,
                equipped_gems: args.equipped_gems,
            });
            if gem_result != InventoryResult::Ok {
                return gem_result;
            }
        }

        InventoryResult::Ok
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
        Self::set_dynamic_update_mask_index(&mut self.active_data.transmog_update_mask, index);
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
        Self::set_dynamic_update_mask_index(&mut self.active_data.transmog_update_mask, slot);
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
        Self::set_dynamic_update_mask_index(
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
        Self::set_dynamic_update_mask_index(
            &mut self.active_data.conditional_transmog_update_mask,
            index,
        );
        self.mark_active_player_data(ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT);
        true
    }

    pub fn conditional_transmog_like_cpp(&self) -> &[i32] {
        &self.active_data.conditional_transmog
    }

    pub const fn is_use_equipped_weapon(
        mainhand: bool,
        is_in_feral_form: bool,
        is_disarmed: bool,
    ) -> bool {
        !is_in_feral_form && (!mainhand || !is_disarmed)
    }
}
