// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Bank slot state and bank storage planning.

use super::super::super::*;

impl Player {
    /// C++ `Player::GetBankBagSlotCount` (`Player.h:1334`).
    pub const fn bank_bag_slot_count(&self) -> u8 {
        self.data.num_bank_slots
    }
    pub fn bank_bag_slot_flag_value_like_cpp(&self, index: usize) -> Option<u32> {
        self.active_data.bank_bag_slot_flags.get(index).copied()
    }

    pub fn set_bank_bag_slot_count(&mut self, count: u8) {
        self.set_player_u8(PLAYER_DATA_NUM_BANK_SLOTS_BIT, count, |data| {
            &mut data.num_bank_slots
        });
    }

    pub fn mark_bank_bag_slot_count_changed_like_cpp(&mut self) {
        self.mark_player_data(PLAYER_DATA_NUM_BANK_SLOTS_BIT);
    }

    pub fn set_bank_bag_slot_flag_value_like_cpp(&mut self, index: usize, value: u32) -> bool {
        if index >= self.active_data.bank_bag_slot_flags.len() {
            return false;
        }

        if self.active_data.bank_bag_slot_flags[index] != value {
            self.active_data.bank_bag_slot_flags[index] = value;
            self.mark_bank_bag_slot_flag_changed_like_cpp(index);
        }
        true
    }

    pub fn mark_bank_bag_slot_flag_changed_like_cpp(&mut self, index: usize) {
        self.mark_active_player_data_array(
            ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_PARENT_BIT,
            ACTIVE_PLAYER_DATA_BANK_BAG_SLOT_FLAGS_FIRST_BIT,
            index,
        );
    }

    pub fn can_bank_item(
        &self,
        dest: &mut Vec<ItemPosCount>,
        args: CanBankItemArgs<'_>,
    ) -> InventoryResult {
        let Some(source) = args.source_item else {
            return if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            };
        };

        let Some(proto) = args.proto else {
            return if args.swap {
                InventoryResult::CantSwap
            } else {
                InventoryResult::ItemNotFound
            };
        };

        if source.loot_generated() {
            return InventoryResult::LootGone;
        }

        if source.is_binded_not_with(self.guid(), proto, args.source_bop_trade_allowed_for_player) {
            return InventoryResult::NotOwner;
        }

        if args.source_is_currency_token {
            return InventoryResult::CantSwap;
        }

        let similar_result = self.can_take_more_similar_items(CanTakeMoreSimilarItemsArgs {
            proto: args.proto,
            count: source.count(),
            source_item: args.source_item,
            current_item_count: self.item_count_by_entry(
                proto.entry,
                true,
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
            return similar_result.result;
        }

        let mut count = source.count();

        if args.bag != NULL_BAG && args.slot != NULL_SLOT {
            if (BANK_SLOT_BAG_START..BANK_SLOT_BAG_END).contains(&args.slot) {
                if !args.source_is_bag {
                    return InventoryResult::WrongSlot;
                }

                if args.slot - BANK_SLOT_BAG_START >= self.data.num_bank_slots {
                    return InventoryResult::NoBankSlot;
                }

                if args.can_use_result != InventoryResult::Ok {
                    return args.can_use_result;
                }
            }

            let result = self.can_store_item_in_specific_slot(
                args.bag,
                args.slot,
                dest,
                proto,
                &mut count,
                args.swap,
                item_ref_by_pos(args.slot_items, args.bag, args.slot),
                args.source_item,
                args.source_is_not_empty_bag,
                bag_template_by_pos(args.bag_templates, args.bag),
            );
            if result != InventoryResult::Ok {
                return result;
            }

            if count == 0 {
                return InventoryResult::Ok;
            }
        }

        if args.bag != NULL_BAG {
            if args.source_is_not_empty_bag {
                return InventoryResult::BagInBag;
            }

            if proto.max_stack_size != 1 {
                if args.bag == INVENTORY_SLOT_BAG_0 {
                    let result = self.can_store_item_in_inventory_slots(
                        BANK_SLOT_ITEM_START,
                        BANK_SLOT_ITEM_END,
                        dest,
                        proto,
                        &mut count,
                        true,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        return result;
                    }
                    if count == 0 {
                        return InventoryResult::Ok;
                    }
                } else {
                    let mut result = self.can_store_item_in_bag(
                        args.bag,
                        dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        NULL_BAG,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, args.bag),
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        result = self.can_store_item_in_bag(
                            args.bag,
                            dest,
                            proto,
                            &mut count,
                            true,
                            true,
                            args.source_item,
                            args.source_is_not_empty_bag,
                            NULL_BAG,
                            args.slot,
                            bag_template_by_pos(args.bag_templates, args.bag),
                            args.slot_items,
                        );
                    }
                    if result != InventoryResult::Ok {
                        return result;
                    }
                    if count == 0 {
                        return InventoryResult::Ok;
                    }
                }
            }

            if args.bag == INVENTORY_SLOT_BAG_0 {
                let result = self.can_store_item_in_inventory_slots(
                    BANK_SLOT_ITEM_START,
                    BANK_SLOT_ITEM_END,
                    dest,
                    proto,
                    &mut count,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    return result;
                }
                if count == 0 {
                    return InventoryResult::Ok;
                }
            } else {
                let mut result = self.can_store_item_in_bag(
                    args.bag,
                    dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    NULL_BAG,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, args.bag),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    result = self.can_store_item_in_bag(
                        args.bag,
                        dest,
                        proto,
                        &mut count,
                        false,
                        true,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        NULL_BAG,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, args.bag),
                        args.slot_items,
                    );
                }
                if result != InventoryResult::Ok {
                    return result;
                }
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        if proto.max_stack_size != 1 {
            let result = self.can_store_item_in_inventory_slots(
                BANK_SLOT_ITEM_START,
                BANK_SLOT_ITEM_END,
                dest,
                proto,
                &mut count,
                true,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                return result;
            }
            if count == 0 {
                return InventoryResult::Ok;
            }

            if !proto.bag_family.is_empty() {
                for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
                    let result = self.can_store_item_in_bag(
                        bag_slot,
                        dest,
                        proto,
                        &mut count,
                        true,
                        false,
                        args.source_item,
                        args.source_is_not_empty_bag,
                        args.bag,
                        args.slot,
                        bag_template_by_pos(args.bag_templates, bag_slot),
                        args.slot_items,
                    );
                    if result != InventoryResult::Ok {
                        continue;
                    }
                    if count == 0 {
                        return InventoryResult::Ok;
                    }
                }
            }

            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
                let result = self.can_store_item_in_bag(
                    bag_slot,
                    dest,
                    proto,
                    &mut count,
                    true,
                    true,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, bag_slot),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    continue;
                }
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        if !proto.bag_family.is_empty() {
            for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
                let result = self.can_store_item_in_bag(
                    bag_slot,
                    dest,
                    proto,
                    &mut count,
                    false,
                    false,
                    args.source_item,
                    args.source_is_not_empty_bag,
                    args.bag,
                    args.slot,
                    bag_template_by_pos(args.bag_templates, bag_slot),
                    args.slot_items,
                );
                if result != InventoryResult::Ok {
                    continue;
                }
                if count == 0 {
                    return InventoryResult::Ok;
                }
            }
        }

        let result = self.can_store_item_in_inventory_slots(
            BANK_SLOT_ITEM_START,
            BANK_SLOT_ITEM_END,
            dest,
            proto,
            &mut count,
            false,
            args.source_item,
            args.source_is_not_empty_bag,
            args.bag,
            args.slot,
            args.slot_items,
        );
        if result != InventoryResult::Ok {
            return result;
        }
        if count == 0 {
            return InventoryResult::Ok;
        }

        for bag_slot in BANK_SLOT_BAG_START..BANK_SLOT_BAG_END {
            let result = self.can_store_item_in_bag(
                bag_slot,
                dest,
                proto,
                &mut count,
                false,
                true,
                args.source_item,
                args.source_is_not_empty_bag,
                args.bag,
                args.slot,
                bag_template_by_pos(args.bag_templates, bag_slot),
                args.slot_items,
            );
            if result != InventoryResult::Ok {
                continue;
            }
            if count == 0 {
                return InventoryResult::Ok;
            }
        }

        InventoryResult::BankFull
    }
}
