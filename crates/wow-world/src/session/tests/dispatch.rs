// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[tokio::test]
async fn time_sync_response_alias_opcodes_dispatch_to_same_handler_like_cpp() {
    for opcode in [
        ClientOpcodes::TimeSyncResponseDropped,
        ClientOpcodes::TimeSyncResponseFailed,
    ] {
        let (mut session, _pkt_tx, _send_rx) = make_session();
        session.set_state(SessionState::LoggedIn);
        let _ = WorldSession::game_time_ms_like_cpp();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let sent_time = WorldSession::game_time_ms_like_cpp();
        session.time_sync_pending_requests.insert(11, sent_time);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(opcode as u16).to_le_bytes());
        bytes.extend_from_slice(&11u32.to_le_bytes());
        bytes.extend_from_slice(&sent_time.saturating_sub(1).to_le_bytes());

        session
            .dispatch_packet(WorldPacket::from_bytes(&bytes))
            .await;

        assert!(
            session.time_sync_pending_requests.is_empty(),
            "{opcode:?} must consume the pending request like C++ HandleTimeSyncResponse"
        );
        assert_eq!(
            session.time_sync_clock_delta_queue.len(),
            1,
            "{opcode:?} must record one clock-delta sample through the shared handler"
        );
    }
}

#[test]
fn reconciled_handler_registrations_match_cpp_metadata_and_rust_targets() {
    let (session, _, _) = make_session();
    let expected = [
        (
            ClientOpcodes::TrainerBuySpell,
            SessionStatus::LoggedIn,
            PacketProcessing::Inplace,
            "handle_trainer_buy_spell",
        ),
        (
            ClientOpcodes::PartyUninvite,
            SessionStatus::LoggedIn,
            PacketProcessing::ThreadUnsafe,
            "handle_party_uninvite",
        ),
        (
            ClientOpcodes::MoveSetVehicleRecIdAck,
            SessionStatus::LoggedIn,
            PacketProcessing::ThreadSafe,
            "handle_move_set_vehicle_rec_id_ack",
        ),
    ];

    for (opcode, status, processing, handler_name) in expected {
        let entry = session
            .dispatch_table
            .get(&opcode)
            .unwrap_or_else(|| panic!("missing linked registration for {opcode:?}"));
        assert_eq!(entry.status, status, "{opcode:?} status");
        assert_eq!(entry.processing, processing, "{opcode:?} processing");
        assert_eq!(entry.handler_name, handler_name, "{opcode:?} target");
    }
}

#[test]
fn dispatch_metadata_matches_cpp_for_registered_active_opcodes() {
    let (session, _, _) = make_session();
    let table = &session.dispatch_table;

    fn status_from_cpp(value: &str) -> Option<SessionStatus> {
        match value {
            "STATUS_AUTHED" => Some(SessionStatus::Authed),
            "STATUS_LOGGEDIN" => Some(SessionStatus::LoggedIn),
            "STATUS_TRANSFER" => Some(SessionStatus::Transfer),
            "STATUS_LOGGEDIN_OR_RECENTLY_LOGGOUT" => Some(SessionStatus::LoggedInOrRecentlyLogout),
            "STATUS_NEVER" | "STATUS_UNHANDLED" => None,
            other => panic!("unknown C++ session status {other}"),
        }
    }

    fn processing_from_cpp(value: &str) -> PacketProcessing {
        match value {
            "PROCESS_INPLACE" => PacketProcessing::Inplace,
            "PROCESS_THREADUNSAFE" => PacketProcessing::ThreadUnsafe,
            "PROCESS_THREADSAFE" => PacketProcessing::ThreadSafe,
            other => panic!("unknown C++ packet processing {other}"),
        }
    }

    let cpp_metadata =
        include_str!("../../../../../docs/migration/inventory/cpp-client-handlers.tsv");
    let mut expected = std::collections::HashMap::new();
    let mut cpp_never_or_unhandled = std::collections::HashSet::new();

    for line in cpp_metadata.lines().skip(1) {
        let columns: Vec<_> = line.split('\t').collect();
        let rust_const = columns[9];
        if rust_const == "-" {
            continue;
        }

        let cpp_status = columns[3];
        let cpp_processing = columns[4];
        if let Some(status) = status_from_cpp(cpp_status) {
            expected.insert(rust_const, (status, processing_from_cpp(cpp_processing)));
        } else {
            cpp_never_or_unhandled.insert(rust_const);
        }
    }

    let compatibility_exceptions = [
        "BattlePayGetPurchaseList",
        "AddBattlenetFriend",
        "BattlenetChallengeResponse",
        "ChangeBagSlotFlag",
        "CloseQuestChoice",
        "ConnectToFailed",
        "GetAccountCharacterList",
        "GetAccountNotifications",
        "LogStreamingError",
        "LogoutInstant",
        "MoveAddImpulseAck",
        "MoveApplyInertiaAck",
        "MoveRemoveInertiaAck",
        "MoveRemoveMovementForces",
        "MoveSeamlessTransferComplete",
        "MoveSetAdvFly",
        "MoveSetAdvFlyingAddImpulseMaxSpeedAck",
        "MoveSetAdvFlyingAirFrictionAck",
        "MoveSetAdvFlyingBankingRateAck",
        "MoveSetAdvFlyingDoubleJumpVelModAck",
        "MoveSetAdvFlyingGlideStartMinHeightAck",
        "MoveSetAdvFlyingLaunchSpeedCoefficientAck",
        "MoveSetAdvFlyingLiftCoefficientAck",
        "MoveSetAdvFlyingMaxVelAck",
        "MoveSetAdvFlyingOverMaxDecelerationAck",
        "MoveSetAdvFlyingPitchingRateDownAck",
        "MoveSetAdvFlyingPitchingRateUpAck",
        "MoveSetAdvFlyingSurfaceFrictionAck",
        "MoveSetAdvFlyingTurnVelocityThresholdAck",
        "OverrideScreenFlash",
        "Ping",
        "QueryQuestItemUsability",
        "QueryCountdownTimer",
        "ReportClientVariables",
        "ReportEnabledAddons",
        "ReportFrozenWhileLoadingMap",
        "ReportKeybindingExecutionCounts",
        "RequestConquestFormulaConstants",
        "SaveAccountDataExport",
        "SetInsertItemsLeftToRight",
        "SetPreferredCemetery",
        "SpawnTrackingUpdate",
        "TimeAdjustmentResponse",
        "UpdateAreaTriggerVisual",
        "UpdateClientSettings",
        "UpdateSpellVisual",
        "UpdateVasPurchaseStates",
        "UsedFollow",
    ];
    let modern_client_non_wotlk_exceptions = [
        "BattlePetClearFanfare",
        "BattlePetRequestJournal",
        "BattlePetRequestJournalLock",
        "BattlePetSetBattleSlot",
        "BattlePetSetFlags",
        "BattlePetSummon",
        "BattlePetUpdateDisplayNotify",
        "BattlePetUpdateNotify",
        "QueryBattlePetName",
    ];

    for entry in table.values() {
        let opcode_name = format!("{:?}", entry.opcode);
        if modern_client_non_wotlk_exceptions.contains(&opcode_name.as_str()) {
            continue;
        }
        if compatibility_exceptions.contains(&opcode_name.as_str()) {
            assert!(
                cpp_never_or_unhandled.contains(opcode_name.as_str()),
                "{opcode_name} is listed as a compatibility exception but C++ metadata is active"
            );
            continue;
        }

        let (status, processing) = expected
            .get(opcode_name.as_str())
            .unwrap_or_else(|| panic!("missing C++ metadata row for {opcode_name}"));
        assert_eq!(entry.status, *status, "{opcode_name} status");
        assert_eq!(entry.processing, *processing, "{opcode_name} processing");
    }
}

#[test]
fn dispatch_table_has_no_duplicate_registered_opcodes() {
    let mut counts = std::collections::HashMap::new();
    for entry in inventory::iter::<PacketHandlerEntry> {
        *counts.entry(entry.opcode).or_insert(0usize) += 1;
    }

    let duplicates: Vec<_> = counts
        .into_iter()
        .filter_map(|(opcode, count)| (count > 1).then_some((opcode, count)))
        .collect();

    assert!(duplicates.is_empty(), "duplicate handlers: {duplicates:?}");
}

/// One row of the frozen dispatch table.
#[cfg(test)]
#[derive(serde::Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct DispatchTableRow {
    opcode: String,
    value: u32,
    status: String,
    processing: String,
    handler: String,
}

/// Every opcode the server can dispatch, with the exact `SessionStatus` and
/// `PacketProcessing` it runs under.
///
/// #359 replaces the dispatcher's opcode match with the registration that
/// already existed. Before that change the two sides had to agree; afterwards
/// there is one declaration per opcode. This enumeration is what proves the
/// swap lost nothing: it is generated from the registry as it stood with the
/// match arms in place, and it must keep matching once they are gone.
///
/// It is an enumeration, not a sample: all 478 rows, compared as a set.
#[test]
fn every_registered_opcode_keeps_its_handler_status_and_processing_like_cpp() {
    let golden: Vec<DispatchTableRow> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/packet-handler-dispatch-table.json"
    ))
    .expect("the dispatch-table fixture is valid JSON");

    let mut actual: Vec<DispatchTableRow> = crate::session::registry::build_dispatch_table()
        .values()
        .map(|entry| DispatchTableRow {
            opcode: format!("{:?}", entry.opcode),
            value: entry.opcode as u32,
            status: format!("{:?}", entry.status),
            processing: format!("{:?}", entry.processing),
            handler: entry.handler_name.to_owned(),
        })
        .collect();
    actual.sort();

    assert_eq!(
        actual.len(),
        golden.len(),
        "the registry gained or lost opcodes; regenerate \
         crates/wow-world/tests/fixtures/packet-handler-dispatch-table.json only after \
         auditing which opcode changed and why"
    );
    for (expected, got) in golden.iter().zip(actual.iter()) {
        assert_eq!(
            expected, got,
            "dispatch metadata changed for {}; an opcode must not silently change its \
             handler, status or processing mode",
            expected.opcode
        );
    }
}
