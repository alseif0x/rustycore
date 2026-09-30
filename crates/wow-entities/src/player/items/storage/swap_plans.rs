// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Inventory item swap planning and orchestration.

use super::super::super::*;

impl Player {
    pub fn swap_item_preflight_plan(
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

    pub fn swap_item_empty_destination_plan(
        &self,
        src: u16,
        dst: u16,
        dst_item_present: bool,
        can_store_result: InventoryResult,
        can_bank_result: InventoryResult,
        can_equip_result: InventoryResult,
        equip_dest: u16,
    ) -> SwapItemEmptyDestinationPlan {
        if dst_item_present {
            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::OccupiedDestination,
            };
        }

        if is_inventory_packed_pos(dst) {
            if can_store_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_store_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::MoveToInventory {
                    quest_added_from_bank: is_bank_packed_pos(src),
                },
            };
        }

        if is_bank_packed_pos(dst) {
            if can_bank_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_bank_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::MoveToBank {
                    quest_removed: true,
                },
            };
        }

        if is_equipment_packed_pos(dst) {
            if can_equip_result != InventoryResult::Ok {
                return SwapItemEmptyDestinationPlan {
                    result: SwapItemEmptyDestinationResult::Error(can_equip_result),
                };
            }

            return SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::Equip {
                    dest: equip_dest,
                    auto_unequip_offhand: true,
                },
            };
        }

        SwapItemEmptyDestinationPlan {
            result: SwapItemEmptyDestinationResult::InvalidDestinationNoop,
        }
    }

    pub fn swap_item_merge_fill_plan(
        &self,
        dst: u16,
        source_is_bag: bool,
        destination_is_bag: bool,
        source_count: u32,
        destination_count: u32,
        source_max_stack_size: u32,
        can_store_result: InventoryResult,
        can_bank_result: InventoryResult,
        can_equip_result: InventoryResult,
        equip_dest: u16,
        is_in_world: bool,
    ) -> SwapItemMergeFillPlan {
        if source_is_bag || destination_is_bag {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::ContinueToRealSwap,
                send_refund_info: false,
            };
        }

        let destination_kind = if is_inventory_packed_pos(dst) {
            Some((
                can_store_result,
                SwapItemMergeFillResult::MoveMergedStackToInventory,
            ))
        } else if is_bank_packed_pos(dst) {
            Some((
                can_bank_result,
                SwapItemMergeFillResult::MoveMergedStackToBank,
            ))
        } else if is_equipment_packed_pos(dst) {
            Some((
                can_equip_result,
                SwapItemMergeFillResult::EquipMergedStack {
                    dest: equip_dest,
                    auto_unequip_offhand: true,
                },
            ))
        } else {
            None
        };

        let Some((validation_result, move_result)) = destination_kind else {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::InvalidDestinationNoop,
                send_refund_info: false,
            };
        };

        if validation_result != InventoryResult::Ok {
            return SwapItemMergeFillPlan {
                result: SwapItemMergeFillResult::ContinueToRealSwap,
                send_refund_info: false,
            };
        }

        if source_count.saturating_add(destination_count) <= source_max_stack_size {
            return SwapItemMergeFillPlan {
                result: move_result,
                send_refund_info: true,
            };
        }

        SwapItemMergeFillPlan {
            result: SwapItemMergeFillResult::PartialFill {
                source_remaining_count: source_count
                    .saturating_add(destination_count)
                    .saturating_sub(source_max_stack_size),
                destination_count: source_max_stack_size,
                send_updates: is_in_world,
            },
            send_refund_info: true,
        }
    }

    pub fn swap_item_real_swap_validation_plan(
        &self,
        src: u16,
        dst: u16,
        source_can_store_result: InventoryResult,
        source_can_bank_result: InventoryResult,
        source_can_equip_result: InventoryResult,
        source_equip_dest: u16,
        source_equip_dest_can_unequip_result: InventoryResult,
        destination_can_store_result: InventoryResult,
        destination_can_bank_result: InventoryResult,
        destination_can_equip_result: InventoryResult,
        destination_equip_dest: u16,
        destination_equip_dest_can_unequip_result: InventoryResult,
    ) -> SwapItemRealSwapValidationPlan {
        let (source_result, source_target) = swap_item_real_swap_target_for_destination(
            dst,
            source_can_store_result,
            source_can_bank_result,
            source_can_equip_result,
            source_equip_dest,
            source_equip_dest_can_unequip_result,
        );
        if source_result != InventoryResult::Ok {
            return SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: source_result,
                    subject: SwapItemRealSwapValidationSubject::Source,
                },
            };
        }

        let (destination_result, destination_target) = swap_item_real_swap_target_for_destination(
            src,
            destination_can_store_result,
            destination_can_bank_result,
            destination_can_equip_result,
            destination_equip_dest,
            destination_equip_dest_can_unequip_result,
        );
        if destination_result != InventoryResult::Ok {
            return SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: destination_result,
                    subject: SwapItemRealSwapValidationSubject::Destination,
                },
            };
        }

        SwapItemRealSwapValidationPlan {
            result: SwapItemRealSwapValidationResult::Continue {
                source_target,
                destination_target,
            },
        }
    }

    pub fn swap_item_bag_exchange_plan(
        &self,
        src: u16,
        dst: u16,
        source_bag: Option<SwapBagRef<'_>>,
        destination_bag: Option<SwapBagRef<'_>>,
    ) -> SwapItemBagExchangePlan {
        let (Some(source_bag), Some(destination_bag)) = (source_bag, destination_bag) else {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            };
        };

        let Some((empty_bag_is_source, empty_bag, full_bag)) =
            (if source_bag.is_empty && !is_bag_pos(src) {
                Some((true, source_bag, destination_bag))
            } else if destination_bag.is_empty && !is_bag_pos(dst) {
                Some((false, destination_bag, source_bag))
            } else {
                None
            })
        else {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            };
        };

        let mut count = 0u8;
        for slot in 0..full_bag.bag_size {
            if let Some(item_ref) = full_bag.items.iter().find(|item| item.slot == slot) {
                if !item_ref.can_go_into_empty_bag {
                    return SwapItemBagExchangePlan {
                        result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
                    };
                }
                count = count.saturating_add(1);
            }
        }

        if count > empty_bag.bag_size {
            return SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Error(InventoryResult::CantSwap),
            };
        }

        let mut moves = Vec::new();
        let mut to_slot = 0u8;
        for slot in 0..full_bag.bag_size {
            if full_bag.items.iter().any(|item| item.slot == slot) {
                moves.push(SwapBagItemMove {
                    from_slot: slot,
                    to_slot,
                });
                to_slot = to_slot.saturating_add(1);
            }
        }

        SwapItemBagExchangePlan {
            result: SwapItemBagExchangeResult::Exchange {
                empty_bag_is_source,
                moves,
            },
        }
    }

    pub fn swap_item_real_swap_execution_plan(
        &self,
        src: u16,
        dst: u16,
        source_target: SwapItemRealSwapTarget,
        destination_target: SwapItemRealSwapTarget,
        ae_loot_view_not_empty: bool,
        source_bag_has_looted_item: bool,
        destination_bag_has_looted_item: bool,
    ) -> SwapItemRealSwapExecutionPlan {
        let [src_bag, src_slot] = src.to_be_bytes();
        let [dst_bag, dst_slot] = dst.to_be_bytes();
        let apply_item_dependent_auras = (src_bag == INVENTORY_SLOT_BAG_0
            && src_slot < INVENTORY_SLOT_BAG_END)
            || (dst_bag == INVENTORY_SLOT_BAG_0 && dst_slot < INVENTORY_SLOT_BAG_END);
        let release_loot = ae_loot_view_not_empty
            && ((is_bag_pos(src) && source_bag_has_looted_item)
                || (is_bag_pos(dst) && destination_bag_has_looted_item));

        SwapItemRealSwapExecutionPlan {
            remove_destination_update: false,
            remove_source_update: false,
            source_target,
            destination_target,
            apply_item_dependent_auras,
            release_loot,
            auto_unequip_offhand: true,
        }
    }

    pub fn swap_item_orchestration_plan(
        &self,
        preflight: SwapItemPreflightPlan,
        empty_destination: Option<SwapItemEmptyDestinationPlan>,
        merge_fill: Option<SwapItemMergeFillPlan>,
        real_swap_validation: Option<SwapItemRealSwapValidationPlan>,
        bag_exchange: Option<SwapItemBagExchangePlan>,
        real_swap_execution: Option<SwapItemRealSwapExecutionPlan>,
    ) -> SwapItemOrchestrationPlan {
        match preflight.result {
            SwapItemPreflightResult::NoSource => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::NoSource,
                };
            }
            SwapItemPreflightResult::ChildRedirect {
                first_src,
                first_dst,
                second_src,
                second_dst,
            } => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::ChildRedirect {
                        first_src,
                        first_dst,
                        second_src,
                        second_dst,
                    },
                };
            }
            SwapItemPreflightResult::Error(result) => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error {
                        result,
                        item_order: SwapItemErrorItemOrder::SourceDestination,
                    },
                };
            }
            SwapItemPreflightResult::Continue => {}
        }

        let Some(empty_destination) = empty_destination else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::EmptyDestination,
                ),
            };
        };
        match empty_destination.result {
            SwapItemEmptyDestinationResult::OccupiedDestination => {}
            SwapItemEmptyDestinationResult::Error(result) => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error {
                        result,
                        item_order: SwapItemErrorItemOrder::SourceOnly,
                    },
                };
            }
            _ => {
                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::EmptyDestination(empty_destination),
                };
            }
        }

        let Some(merge_fill) = merge_fill else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(SwapItemMissingPhase::MergeFill),
            };
        };
        if merge_fill.result != SwapItemMergeFillResult::ContinueToRealSwap {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MergeFill(merge_fill),
            };
        }

        let Some(real_swap_validation) = real_swap_validation else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::RealSwapValidation,
                ),
            };
        };
        let (source_target, destination_target) = match real_swap_validation.result {
            SwapItemRealSwapValidationResult::Error { result, subject } => {
                let item_order = match subject {
                    SwapItemRealSwapValidationSubject::Source => {
                        SwapItemErrorItemOrder::SourceDestination
                    }
                    SwapItemRealSwapValidationSubject::Destination => {
                        SwapItemErrorItemOrder::DestinationSource
                    }
                };

                return SwapItemOrchestrationPlan {
                    result: SwapItemOrchestrationResult::Error { result, item_order },
                };
            }
            SwapItemRealSwapValidationResult::Continue {
                source_target,
                destination_target,
            } => (source_target, destination_target),
        };

        let Some(bag_exchange) = bag_exchange else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::BagExchange,
                ),
            };
        };
        if let SwapItemBagExchangeResult::Error(result) = &bag_exchange.result {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::Error {
                    result: *result,
                    item_order: SwapItemErrorItemOrder::SourceDestination,
                },
            };
        }

        let Some(real_swap_execution) = real_swap_execution else {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::MissingPhase(
                    SwapItemMissingPhase::RealSwapExecution,
                ),
            };
        };
        if real_swap_execution.source_target != source_target
            || real_swap_execution.destination_target != destination_target
        {
            return SwapItemOrchestrationPlan {
                result: SwapItemOrchestrationResult::InconsistentRealSwapTargets {
                    validation_source_target: source_target,
                    validation_destination_target: destination_target,
                    execution_source_target: real_swap_execution.source_target,
                    execution_destination_target: real_swap_execution.destination_target,
                },
            };
        }

        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::RealSwap {
                bag_exchange,
                execution: real_swap_execution,
            },
        }
    }

}
