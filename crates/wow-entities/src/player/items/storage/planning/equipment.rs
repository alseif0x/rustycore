use super::ItemStorageView;
use crate::ItemStorageTemplate;
use crate::player::{
    CanEquipItemArgs, CanEquipItemOutcome, CanEquipUniqueItemArgs, CanEquipUniqueItemTemplateArgs,
    CanTakeMoreSimilarItemsArgs, CanUnequipItemArgs, EQUIPMENT_SLOT_MAINHAND,
    EQUIPMENT_SLOT_OFFHAND, FindEquipSlotArgs, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_END,
    INVENTORY_SLOT_BAG_START, NULL_SLOT, SwapItemPreflightItem, SwapItemPreflightPlan,
    SwapItemPreflightResult, can_equip_item_outcome, equip_slot_candidates,
    equipped_gem_limit_category_count, equipped_item_limit_category_count, has_equipped_gem_entry,
    has_equipped_item_entry, is_bag_pos, is_equipment_packed_pos, item_ref_by_pos, make_item_pos,
    paired_unique_ignore_slot,
};
use wow_constants::{
    InventoryResult, InventoryType, ItemClass, ItemSubClassQuiver, ItemSubClassWeapon,
};

impl ItemStorageView<'_> {
    pub(in crate::player::items::storage) fn find_equip_slot(
        &self,
        args: FindEquipSlotArgs<'_>,
    ) -> u8 {
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

    pub(in crate::player::items::storage) fn can_equip_item(
        &self,
        args: CanEquipItemArgs<'_>,
    ) -> CanEquipItemOutcome {
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

        if source.is_binded_not_with(
            self.owner_guid,
            proto,
            args.source_bop_trade_allowed_for_player,
        ) {
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

    pub(in crate::player::items::storage) fn can_unequip_item(
        &self,
        args: CanUnequipItemArgs<'_>,
    ) -> InventoryResult {
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

    pub(in crate::player::items::storage) fn can_equip_unique_item_template(
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

    pub(in crate::player::items::storage) fn can_equip_unique_item(
        &self,
        args: CanEquipUniqueItemArgs<'_>,
    ) -> InventoryResult {
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

    pub(in crate::player::items::storage) fn is_two_hand_used_template(
        &self,
        main_template: Option<&ItemStorageTemplate>,
        can_titan_grip: bool,
    ) -> bool {
        let Some(template) = main_template else {
            return false;
        };

        (template.inventory_type == InventoryType::Weapon2Hand && !can_titan_grip)
            || template.inventory_type == InventoryType::Ranged
            || (template.inventory_type == InventoryType::RangedRight
                && template.class_id == ItemClass::Weapon
                && template.subclass_id != ItemSubClassWeapon::Wand as u32)
    }

    pub(in crate::player::items::storage) fn swap_item_preflight_plan(
        &self,
        src: u16,
        dst: u16,
        is_alive: bool,
        src_item: Option<SwapItemPreflightItem>,
        dst_item: Option<SwapItemPreflightItem>,
    ) -> SwapItemPreflightPlan {
        let Some(src_item) = src_item else {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::NoSource,
                src_unequip_swap: None,
                dst_unequip_swap: None,
            };
        };

        if src_item.is_child {
            if let Some(parent_pos) = src_item.parent_pos {
                if is_equipment_packed_pos(src) {
                    return SwapItemPreflightPlan {
                        result: SwapItemPreflightResult::ChildRedirect {
                            first_src: dst,
                            first_dst: src,
                            second_src: parent_pos,
                            second_dst: dst,
                        },
                        src_unequip_swap: None,
                        dst_unequip_swap: None,
                    };
                }
            }
        } else if let Some(dst_item) = dst_item {
            if dst_item.is_child {
                if let Some(parent_pos) = dst_item.parent_pos {
                    if is_equipment_packed_pos(dst) {
                        return SwapItemPreflightPlan {
                            result: SwapItemPreflightResult::ChildRedirect {
                                first_src: src,
                                first_dst: dst,
                                second_src: parent_pos,
                                second_dst: src,
                            },
                            src_unequip_swap: None,
                            dst_unequip_swap: None,
                        };
                    }
                }
            }
        }

        if !is_alive {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
                src_unequip_swap: None,
                dst_unequip_swap: None,
            };
        }

        let mut src_unequip_swap = None;
        if is_equipment_packed_pos(src) || is_bag_pos(src) {
            let swap = !is_bag_pos(src)
                || is_bag_pos(dst)
                || dst_item.is_some_and(|item| item.is_bag && item.is_empty_bag);
            src_unequip_swap = Some(swap);
            if src_item.can_unequip_result != InventoryResult::Ok {
                return SwapItemPreflightPlan {
                    result: SwapItemPreflightResult::Error(src_item.can_unequip_result),
                    src_unequip_swap,
                    dst_unequip_swap: None,
                };
            }
        }

        let [_src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, _dst_slot] = dst.to_be_bytes();
        if is_bag_pos(src) && src_slot == dst_bag {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::BagInBag),
                src_unequip_swap,
                dst_unequip_swap: None,
            };
        }

        let [src_bag, _src_slot] = src.to_be_bytes();
        let [_dst_bag, dst_slot] = dst.to_be_bytes();
        if is_bag_pos(dst) && src_bag == dst_slot {
            return SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::CantSwap),
                src_unequip_swap,
                dst_unequip_swap: None,
            };
        }

        let mut dst_unequip_swap = None;
        if let Some(dst_item) = dst_item {
            if is_equipment_packed_pos(dst) || is_bag_pos(dst) {
                let swap = !is_bag_pos(dst)
                    || is_bag_pos(src)
                    || (src_item.is_bag && src_item.is_empty_bag);
                dst_unequip_swap = Some(swap);
                if dst_item.can_unequip_result != InventoryResult::Ok {
                    return SwapItemPreflightPlan {
                        result: SwapItemPreflightResult::Error(dst_item.can_unequip_result),
                        src_unequip_swap,
                        dst_unequip_swap,
                    };
                }
            }
        }

        SwapItemPreflightPlan {
            result: SwapItemPreflightResult::Continue,
            src_unequip_swap,
            dst_unequip_swap,
        }
    }
}
