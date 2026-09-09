//! Gameobject scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn swap_item_orchestration_plan_matches_cpp_branch_order() {
    let player = Player::new(None, false);
    let continue_preflight = SwapItemPreflightPlan {
        result: SwapItemPreflightResult::Continue,
        src_unequip_swap: None,
        dst_unequip_swap: None,
    };
    let occupied_destination = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::OccupiedDestination,
    };
    let continue_merge = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::ContinueToRealSwap,
        send_refund_info: false,
    };
    let inventory_bank_validation = SwapItemRealSwapValidationPlan {
        result: SwapItemRealSwapValidationResult::Continue {
            source_target: SwapItemRealSwapTarget::Inventory,
            destination_target: SwapItemRealSwapTarget::Bank,
        },
    };
    let no_bag_exchange = SwapItemBagExchangePlan {
        result: SwapItemBagExchangeResult::Continue,
    };
    let execution = SwapItemRealSwapExecutionPlan {
        remove_destination_update: false,
        remove_source_update: false,
        source_target: SwapItemRealSwapTarget::Inventory,
        destination_target: SwapItemRealSwapTarget::Bank,
        apply_item_dependent_auras: false,
        release_loot: false,
        auto_unequip_offhand: true,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(
            SwapItemPreflightPlan {
                result: SwapItemPreflightResult::Error(InventoryResult::PlayerDead),
                src_unequip_swap: None,
                dst_unequip_swap: None,
            },
            None,
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::PlayerDead,
                item_order: SwapItemErrorItemOrder::SourceDestination,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(SwapItemEmptyDestinationPlan {
                result: SwapItemEmptyDestinationResult::Error(InventoryResult::InvFull),
            }),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::InvFull,
                item_order: SwapItemErrorItemOrder::SourceOnly,
            },
        }
    );

    let move_to_bank = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::MoveToBank {
            quest_removed: true,
        },
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(move_to_bank),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::EmptyDestination(move_to_bank),
        }
    );

    let partial_fill = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::PartialFill {
            source_remaining_count: 2,
            destination_count: 20,
            send_updates: true,
        },
        send_refund_info: true,
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(partial_fill),
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MergeFill(partial_fill),
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(SwapItemRealSwapValidationPlan {
                result: SwapItemRealSwapValidationResult::Error {
                    result: InventoryResult::CantEquipEver,
                    subject: SwapItemRealSwapValidationSubject::Destination,
                },
            }),
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::CantEquipEver,
                item_order: SwapItemErrorItemOrder::DestinationSource,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(inventory_bank_validation),
            Some(SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Error(InventoryResult::BagInBag),
            }),
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::Error {
                result: InventoryResult::BagInBag,
                item_order: SwapItemErrorItemOrder::SourceDestination,
            },
        }
    );

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(inventory_bank_validation),
            Some(no_bag_exchange.clone()),
            Some(execution),
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::RealSwap {
                bag_exchange: no_bag_exchange,
                execution,
            },
        }
    );
}
#[test]
fn swap_item_orchestration_plan_keeps_phase_gaps_visible() {
    let player = Player::new(None, false);
    let continue_preflight = SwapItemPreflightPlan {
        result: SwapItemPreflightResult::Continue,
        src_unequip_swap: None,
        dst_unequip_swap: None,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(continue_preflight, None, None, None, None, None),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MissingPhase(
                SwapItemMissingPhase::EmptyDestination,
            ),
        }
    );

    let occupied_destination = SwapItemEmptyDestinationPlan {
        result: SwapItemEmptyDestinationResult::OccupiedDestination,
    };
    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            None,
            None,
            None,
            None,
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::MissingPhase(SwapItemMissingPhase::MergeFill),
        }
    );

    let continue_merge = SwapItemMergeFillPlan {
        result: SwapItemMergeFillResult::ContinueToRealSwap,
        send_refund_info: false,
    };
    let validation = SwapItemRealSwapValidationPlan {
        result: SwapItemRealSwapValidationResult::Continue {
            source_target: SwapItemRealSwapTarget::Inventory,
            destination_target: SwapItemRealSwapTarget::Bank,
        },
    };
    let mismatched_execution = SwapItemRealSwapExecutionPlan {
        remove_destination_update: false,
        remove_source_update: false,
        source_target: SwapItemRealSwapTarget::Bank,
        destination_target: SwapItemRealSwapTarget::Inventory,
        apply_item_dependent_auras: false,
        release_loot: false,
        auto_unequip_offhand: true,
    };

    assert_eq!(
        player.swap_item_orchestration_plan(
            continue_preflight,
            Some(occupied_destination),
            Some(continue_merge),
            Some(validation),
            Some(SwapItemBagExchangePlan {
                result: SwapItemBagExchangeResult::Continue,
            }),
            Some(mismatched_execution),
        ),
        SwapItemOrchestrationPlan {
            result: SwapItemOrchestrationResult::InconsistentRealSwapTargets {
                validation_source_target: SwapItemRealSwapTarget::Inventory,
                validation_destination_target: SwapItemRealSwapTarget::Bank,
                execution_source_target: SwapItemRealSwapTarget::Bank,
                execution_destination_target: SwapItemRealSwapTarget::Inventory,
            },
        }
    );
}
