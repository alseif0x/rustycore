// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Production-linked handler registry contract, without cfg(test) registrations leaking into the contract.

const CONTRACT_SNAPSHOT: &str =
    include_str!("../../../tools/architecture/world-handler-contract.tsv");
const CONTRACT_HEADER: &str =
    "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing";

fn expected_contract_rows() -> Vec<String> {
    let mut lines = CONTRACT_SNAPSHOT
        .lines()
        .filter(|line| !line.starts_with('#'));
    assert_eq!(
        lines.next(),
        Some(CONTRACT_HEADER),
        "world handler contract header changed unexpectedly"
    );
    lines.map(str::to_owned).collect()
}

fn production_linked_contract_rows() -> Vec<String> {
    // The public Server factory exposes the canonical table used to construct sessions.
    // This integration target links normal libraries, without cfg(test) registrations.
    let _ = std::any::TypeId::of::<wow_world::WorldSession>();

    let registry = world_server::compose_packet_handlers_like_cpp()
        .expect("valid world-server packet handler composition");
    let bank = registry
        .get(wow_constants::ClientOpcodes::ChangeBankBagSlotFlag)
        .expect("normal composition includes Bank registrar");
    assert_eq!(bank.handler_name, "handle_change_bank_bag_slot_flag");
    assert_eq!(bank.status, wow_handler::SessionStatus::LoggedIn);
    assert_eq!(bank.processing, wow_handler::PacketProcessing::Inplace);
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::ChangeBankBagSlotFlag)
            .count(),
        1
    );
    let item_text = registry
        .get(wow_constants::ClientOpcodes::ItemTextQuery)
        .expect("normal composition includes Inventory ItemTextQuery");
    assert_eq!(item_text.handler_name, "handle_item_text_query");
    assert_eq!(item_text.status, wow_handler::SessionStatus::LoggedIn);
    assert_eq!(item_text.processing, wow_handler::PacketProcessing::Inplace);
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::ItemTextQuery)
            .count(),
        1
    );
    let mut rows: Vec<_> = registry
        .iter()
        .map(|entry| {
            (
                entry.opcode as u32,
                format!("{:?}", entry.opcode),
                entry.handler_name,
                format!("{:?}", entry.status),
                format!("{:?}", entry.processing),
            )
        })
        .collect();
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(right.2))
    });
    rows.into_iter()
        .map(
            |(opcode_value, opcode_name, handler_name, status, processing)| {
                format!(
                    "0x{opcode_value:04X}\t{opcode_name}\t{handler_name}\t{status}\t{processing}"
                )
            },
        )
        .collect()
}

#[test]
fn production_linked_world_handler_registry_matches_snapshot() {
    assert_eq!(
        production_linked_contract_rows(),
        expected_contract_rows(),
        "the production-linked registry differs from the reviewed handler contract; \
         do not refresh the snapshot until cfg gating and C++ metadata are audited"
    );
}

/// The trainer tail (#1263 F5) must reach the production-linked registry with
/// its exact reviewed tuple, exactly once, from its explicit area registrar.
#[test]
fn production_linked_registry_carries_the_migrated_trainer_tail() {
    let registry = world_server::compose_packet_handlers_like_cpp()
        .expect("valid world-server packet handler composition");
    for (opcode, handler_name) in [
        (
            wow_constants::ClientOpcodes::TrainerList,
            "handle_trainer_list",
        ),
        (
            wow_constants::ClientOpcodes::TrainerBuySpell,
            "handle_trainer_buy_spell",
        ),
    ] {
        let matches: Vec<_> = registry
            .iter()
            .filter(|entry| entry.opcode == opcode)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{opcode:?} must be registered exactly once"
        );
        assert_eq!(matches[0].handler_name, handler_name);
        assert_eq!(matches[0].status, wow_handler::SessionStatus::LoggedIn);
        assert_eq!(
            matches[0].processing,
            wow_handler::PacketProcessing::Inplace
        );
    }
}

/// The movement registration tail (#1263 F5 tail) must reach the
/// production-linked registry: all 54 effective entries exactly once, with the
/// reviewed handler names and metadata, from its explicit area registrar.
#[test]
fn production_linked_registry_carries_the_migrated_movement_tail() {
    let registry = world_server::compose_packet_handlers_like_cpp()
        .expect("valid world-server packet handler composition");
    let expected: &[(wow_constants::ClientOpcodes, &str)] = &[
        (
            wow_constants::ClientOpcodes::MoveStartForward,
            "handle_movement_MoveStartForward",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartBackward,
            "handle_movement_MoveStartBackward",
        ),
        (
            wow_constants::ClientOpcodes::MoveStop,
            "handle_movement_MoveStop",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartStrafeLeft,
            "handle_movement_MoveStartStrafeLeft",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartStrafeRight,
            "handle_movement_MoveStartStrafeRight",
        ),
        (
            wow_constants::ClientOpcodes::MoveStopStrafe,
            "handle_movement_MoveStopStrafe",
        ),
        (
            wow_constants::ClientOpcodes::MoveJump,
            "handle_movement_MoveJump",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartTurnLeft,
            "handle_movement_MoveStartTurnLeft",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartTurnRight,
            "handle_movement_MoveStartTurnRight",
        ),
        (
            wow_constants::ClientOpcodes::MoveStopTurn,
            "handle_movement_MoveStopTurn",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartPitchUp,
            "handle_movement_MoveStartPitchUp",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartPitchDown,
            "handle_movement_MoveStartPitchDown",
        ),
        (
            wow_constants::ClientOpcodes::MoveStopPitch,
            "handle_movement_MoveStopPitch",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetRunMode,
            "handle_movement_MoveSetRunMode",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetWalkMode,
            "handle_movement_MoveSetWalkMode",
        ),
        (
            wow_constants::ClientOpcodes::MoveFallLand,
            "handle_movement_MoveFallLand",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartSwim,
            "handle_movement_MoveStartSwim",
        ),
        (
            wow_constants::ClientOpcodes::MoveStopSwim,
            "handle_movement_MoveStopSwim",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetFacing,
            "handle_movement_MoveSetFacing",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetPitch,
            "handle_movement_MoveSetPitch",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceRunSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceRunBackSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceSwimSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceRootAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceUnrootAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveHeartbeat,
            "handle_movement_MoveHeartbeat",
        ),
        (
            wow_constants::ClientOpcodes::MoveHoverAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSplineDone,
            "handle_move_spline_done",
        ),
        (
            wow_constants::ClientOpcodes::MoveFallReset,
            "handle_movement_MoveFallReset",
        ),
        (
            wow_constants::ClientOpcodes::MoveUpdateFallSpeed,
            "handle_movement_MoveUpdateFallSpeed",
        ),
        (
            wow_constants::ClientOpcodes::MoveFeatherFallAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveWaterWalkAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveEnableDoubleJumpAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceWalkSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceSwimBackSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceTurnRateChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveEnableSwimToFlyTransAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetCanTurnWhileFallingAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetIgnoreMovementForcesAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetCanFlyAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetFly,
            "handle_movement_MoveSetFly",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartAscend,
            "handle_movement_MoveStartAscend",
        ),
        (
            wow_constants::ClientOpcodes::MoveStopAscend,
            "handle_movement_MoveStopAscend",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceFlightSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveForceFlightBackSpeedChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveStartDescend,
            "handle_movement_MoveStartDescend",
        ),
        (
            wow_constants::ClientOpcodes::MoveForcePitchRateChangeAck,
            "handle_movement_speed_ack",
        ),
        (
            wow_constants::ClientOpcodes::MoveGravityDisableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveGravityEnableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveInertiaDisableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveInertiaEnableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveCollisionDisableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveCollisionEnableAck,
            "handle_movement_ack_message",
        ),
        (
            wow_constants::ClientOpcodes::MoveSetFacingHeartbeat,
            "handle_movement_MoveSetFacingHeartbeat",
        ),
    ];
    assert_eq!(
        expected.len(),
        54,
        "the tail carries 53 macro + 1 direct entries"
    );
    for (opcode, handler_name) in expected {
        let matches: Vec<_> = registry
            .iter()
            .filter(|entry| entry.opcode == *opcode)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{opcode:?} must be registered exactly once"
        );
        assert_eq!(
            matches[0].handler_name, *handler_name,
            "{opcode:?} handler name"
        );
        assert_eq!(matches[0].status, wow_handler::SessionStatus::LoggedIn);
        assert_eq!(
            matches[0].processing,
            wow_handler::PacketProcessing::ThreadSafe
        );
    }
}

/// The character/account registration family (#1263 F5 remaining families) must
/// reach the production-linked registry: all 23 effective entries exactly once,
/// with the reviewed handler names and metadata, from its explicit area
/// registrar.
#[test]
fn production_linked_registry_carries_the_migrated_character_account_family() {
    let registry = world_server::compose_packet_handlers_like_cpp()
        .expect("valid world-server packet handler composition");
    // (opcode, handler_name, processing)
    let expected: &[(
        wow_constants::ClientOpcodes,
        &str,
        wow_handler::PacketProcessing,
    )] = &[
        (
            wow_constants::ClientOpcodes::ListInventory,
            "handle_list_inventory",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::BuyItem,
            "handle_buy_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::BuyBackItem,
            "handle_buy_back_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::SellItem,
            "handle_sell_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::ItemPurchaseRefund,
            "handle_item_purchase_refund",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::BankerActivate,
            "handle_banker_activate",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::AutobankItem,
            "handle_autobank_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::AutostoreBankItem,
            "handle_autostore_bank_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::BuyBankSlot,
            "handle_buy_bank_slot",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::BinderActivate,
            "handle_binder_activate",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::HearthAndResurrect,
            "handle_hearth_and_resurrect",
            wow_handler::PacketProcessing::ThreadUnsafe,
        ),
        (
            wow_constants::ClientOpcodes::RepairItem,
            "handle_repair_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::QuestGiverStatusMultipleQuery,
            "handle_quest_giver_status_multiple_query",
            wow_handler::PacketProcessing::ThreadUnsafe,
        ),
        (
            wow_constants::ClientOpcodes::QuestGiverStatusTrackedQuery,
            "handle_quest_giver_status_tracked_query",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::SwapInvItem,
            "handle_swap_inv_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::AutoEquipItem,
            "handle_auto_equip_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::AutoEquipItemSlot,
            "handle_auto_equip_item_slot",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::SwapItem,
            "handle_swap_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::AutoStoreBagItem,
            "handle_auto_store_bag_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::DestroyItem,
            "handle_destroy_item",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::TalkToGossip,
            "handle_gossip_hello",
            wow_handler::PacketProcessing::Inplace,
        ),
        (
            wow_constants::ClientOpcodes::GossipSelectOption,
            "handle_gossip_select_option",
            wow_handler::PacketProcessing::ThreadUnsafe,
        ),
        (
            wow_constants::ClientOpcodes::LogoutRequest,
            "handle_logout_request",
            wow_handler::PacketProcessing::ThreadUnsafe,
        ),
    ];
    assert_eq!(
        expected.len(),
        23,
        "the character/account family carries 23 effective entries"
    );
    for (opcode, handler_name, processing) in expected {
        let matches: Vec<_> = registry
            .iter()
            .filter(|entry| entry.opcode == *opcode)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{opcode:?} must be registered exactly once"
        );
        assert_eq!(
            matches[0].handler_name, *handler_name,
            "{opcode:?} handler name"
        );
        assert_eq!(matches[0].status, wow_handler::SessionStatus::LoggedIn);
        assert_eq!(matches[0].processing, *processing, "{opcode:?} processing");
    }
}
