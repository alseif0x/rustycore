// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Regression guard for the complete world-handler registration contract.
//!
//! The dispatcher is assembled through `inventory`, so checking source files or
//! registration counts alone cannot prove which handlers are linked into the
//! final `wow-world` test binary. This module snapshots the effective registry:
//! opcode value/name, handler name, session status, and processing mode.

use crate::session::registry::PacketHandlerEntry;

const CONTRACT_SNAPSHOT: &str =
    include_str!("../../../tools/architecture/world-handler-contract.tsv");
const CONTRACT_HEADER: &str =
    "opcode_value\topcode_name\thandler_name\tsession_status\tpacket_processing";

#[derive(Clone, Debug, Eq, PartialEq)]
struct HandlerContractRow {
    opcode_value: u32,
    opcode_name: String,
    handler_name: String,
    session_status: String,
    packet_processing: String,
}

impl HandlerContractRow {
    fn from_entry(entry: &PacketHandlerEntry) -> Self {
        Self {
            opcode_value: entry.opcode as u32,
            opcode_name: format!("{:?}", entry.opcode),
            handler_name: entry.handler_name.to_owned(),
            session_status: format!("{:?}", entry.status),
            packet_processing: format!("{:?}", entry.processing),
        }
    }

    fn display(&self) -> String {
        format!(
            "0x{:04X} {} handler={} status={} processing={}",
            self.opcode_value,
            self.opcode_name,
            self.handler_name,
            self.session_status,
            self.packet_processing
        )
    }
}

fn registered_contract() -> Vec<HandlerContractRow> {
    let mut rows: Vec<_> = crate::session::registry::registered_handler_entries_like_cpp()
        .map(|entry| HandlerContractRow::from_entry(&entry))
        .collect();
    rows.sort_by(|left, right| {
        left.opcode_value
            .cmp(&right.opcode_value)
            .then_with(|| left.opcode_name.cmp(&right.opcode_name))
            .then_with(|| left.handler_name.cmp(&right.handler_name))
    });
    rows
}

fn render_contract(rows: &[HandlerContractRow]) -> String {
    let mut rendered = String::from(
        "# Generated from the linked inventory registry; do not hand-edit rows.\n\
         # Refresh deliberately with: cargo test -p wow-world print_world_handler_contract_snapshot --lib -- --ignored --nocapture\n",
    );
    rendered.push_str(CONTRACT_HEADER);
    rendered.push('\n');
    for row in rows {
        rendered.push_str(&format!(
            "0x{:04X}\t{}\t{}\t{}\t{}\n",
            row.opcode_value,
            row.opcode_name,
            row.handler_name,
            row.session_status,
            row.packet_processing
        ));
    }
    rendered
}

fn parse_contract(snapshot: &str) -> Result<Vec<HandlerContractRow>, String> {
    let mut lines = snapshot
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'));
    let Some((header_line, header)) = lines.next() else {
        return Err("handler contract snapshot has no header".to_owned());
    };
    if header != CONTRACT_HEADER {
        return Err(format!(
            "handler contract header at line {}: expected {CONTRACT_HEADER:?}, actual {header:?}",
            header_line + 1
        ));
    }

    let mut rows = Vec::new();
    for (line_index, line) in lines {
        if line.is_empty() {
            return Err(format!(
                "handler contract contains an empty row at line {}",
                line_index + 1
            ));
        }
        let columns: Vec<_> = line.split('\t').collect();
        let [
            opcode_value,
            opcode_name,
            handler_name,
            session_status,
            packet_processing,
        ] = columns.as_slice()
        else {
            return Err(format!(
                "handler contract line {} has {} columns; expected 5",
                line_index + 1,
                columns.len()
            ));
        };
        let opcode_value = opcode_value
            .strip_prefix("0x")
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .ok_or_else(|| {
                format!(
                    "handler contract line {} has invalid opcode value {opcode_value:?}",
                    line_index + 1
                )
            })?;
        rows.push(HandlerContractRow {
            opcode_value,
            opcode_name: (*opcode_name).to_owned(),
            handler_name: (*handler_name).to_owned(),
            session_status: (*session_status).to_owned(),
            packet_processing: (*packet_processing).to_owned(),
        });
    }
    Ok(rows)
}

fn compare_contract(
    expected: &[HandlerContractRow],
    actual: &[HandlerContractRow],
) -> Result<(), String> {
    for index in 0..expected.len().max(actual.len()) {
        let row_number = index + 1;
        let Some(expected_row) = expected.get(index) else {
            return Err(format!(
                "handler contract row {row_number}: unexpected actual {}",
                actual[index].display()
            ));
        };
        let Some(actual_row) = actual.get(index) else {
            return Err(format!(
                "handler contract row {row_number}: missing actual {}",
                expected_row.display()
            ));
        };

        let fields = [
            (
                "opcode_value",
                format!("0x{:04X}", expected_row.opcode_value),
                format!("0x{:04X}", actual_row.opcode_value),
            ),
            (
                "opcode_name",
                expected_row.opcode_name.clone(),
                actual_row.opcode_name.clone(),
            ),
            (
                "handler_name",
                expected_row.handler_name.clone(),
                actual_row.handler_name.clone(),
            ),
            (
                "session_status",
                expected_row.session_status.clone(),
                actual_row.session_status.clone(),
            ),
            (
                "packet_processing",
                expected_row.packet_processing.clone(),
                actual_row.packet_processing.clone(),
            ),
        ];
        if let Some((field, expected_value, actual_value)) = fields
            .into_iter()
            .find(|(_, expected_value, actual_value)| expected_value != actual_value)
        {
            return Err(format!(
                "handler contract row {row_number} ({}) field {field}: expected {expected_value:?}, actual {actual_value:?}",
                expected_row.opcode_name
            ));
        }
    }
    Ok(())
}

fn contract_row(
    opcode_value: u32,
    opcode_name: &str,
    handler_name: &str,
    session_status: &str,
    packet_processing: &str,
) -> HandlerContractRow {
    HandlerContractRow {
        opcode_value,
        opcode_name: opcode_name.to_owned(),
        handler_name: handler_name.to_owned(),
        session_status: session_status.to_owned(),
        packet_processing: packet_processing.to_owned(),
    }
}

#[test]
fn world_handler_contract_matches_snapshot() {
    let expected = parse_contract(CONTRACT_SNAPSHOT)
        .unwrap_or_else(|error| panic!("invalid world handler contract snapshot: {error}"));
    let actual = registered_contract();

    if let Err(error) = compare_contract(&expected, &actual) {
        panic!(
            "{error} (expected {} rows, actual {})\n\
             update tools/architecture/world-handler-contract.tsv only after auditing the \
             opcode, handler, status, and processing change",
            expected.len(),
            actual.len()
        );
    }
}

#[test]
fn contract_comparison_reports_handler_name_and_metadata_drift() {
    let expected = vec![contract_row(
        0x1234,
        "ExampleOpcode",
        "handle_example",
        "LoggedIn",
        "ThreadUnsafe",
    )];

    for (actual, expected_field) in [
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_other",
                "LoggedIn",
                "ThreadUnsafe",
            )],
            "field handler_name",
        ),
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_example",
                "Authed",
                "ThreadUnsafe",
            )],
            "field session_status",
        ),
        (
            vec![contract_row(
                0x1234,
                "ExampleOpcode",
                "handle_example",
                "LoggedIn",
                "Inplace",
            )],
            "field packet_processing",
        ),
    ] {
        let error =
            compare_contract(&expected, &actual).expect_err("contract drift must be reported");
        assert!(
            error.contains(expected_field),
            "expected {expected_field:?} in drift report, got {error:?}"
        );
    }
}

/// Prints the canonical snapshot to stdout for a deliberate, reviewable refresh.
///
/// Copy only the text between the markers into
/// `tools/architecture/world-handler-contract.tsv`, audit the diff, and run the
/// non-ignored contract test. The test never overwrites the baseline itself.
#[test]
#[ignore = "manual snapshot refresh helper"]
fn print_world_handler_contract_snapshot() {
    println!(
        "----- BEGIN world-handler-contract.tsv -----\n{}----- END world-handler-contract.tsv -----",
        render_contract(&registered_contract())
    );
}

/// The rows the legacy inventory drain still contributes (#1263 F5 tail).
///
/// These are the registrations `register_remaining_handlers_like_cpp` links from
/// `inventory::submit!`; every migrated family must be absent from this set, so
/// a family that silently kept its legacy submission fails here.
fn legacy_inventory_contract_rows() -> Vec<HandlerContractRow> {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    crate::session::registry::register_remaining_handlers_like_cpp(&mut builder)
        .expect("legacy inventory drain registers");
    builder
        .build()
        .iter()
        .map(HandlerContractRow::from_entry)
        .collect()
}

/// Assert one migrated family: exact tuples, one registration each, and no
/// legacy submission left behind.
fn assert_migrated_family_like_cpp(
    family: &str,
    registrar: impl FnOnce(&mut crate::session::registry::WorldPacketHandlerRegistryBuilder),
    expected: &[HandlerContractRow],
    expected_legacy_rows: usize,
) {
    let composed = registered_contract();
    for row in expected {
        let matches: Vec<_> = composed
            .iter()
            .filter(|candidate| candidate.opcode_value == row.opcode_value)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "{family}: {} must be registered exactly once in the composed registry",
            row.display()
        );
        assert_eq!(
            *matches[0],
            *row,
            "{family}: {} changed its registered tuple",
            row.display()
        );
    }

    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    registrar(&mut builder);
    let registrar_rows: Vec<_> = builder
        .build()
        .iter()
        .map(HandlerContractRow::from_entry)
        .collect();
    for row in expected {
        assert!(
            registrar_rows.contains(row),
            "{family}: {} must originate from its explicit area registrar",
            row.display()
        );
    }

    let legacy = legacy_inventory_contract_rows();
    for row in expected {
        assert!(
            !legacy
                .iter()
                .any(|candidate| candidate.opcode_value == row.opcode_value),
            "{family}: {} is still submitted through the legacy inventory",
            row.display()
        );
    }
    assert_eq!(
        legacy.len(),
        expected_legacy_rows,
        "{family}: the legacy inventory drain must hold exactly the measured residual rows"
    );
}

/// Negative control (#1263 F5 tail): re-registering the migrated tail on the
/// same builder is rejected and never replaces the first entry.
#[test]
fn duplicated_migrated_tail_registration_is_rejected_without_replacement() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_movement_tail_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect("the first tail registration succeeds");
    let error = wow_world_application::register_movement_tail_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect_err("a second tail registration must be rejected");
    assert_eq!(error.opcode, wow_constants::ClientOpcodes::MoveStartForward);
    assert_eq!(
        error.previous_handler_name,
        "handle_movement_MoveStartForward"
    );
    assert_eq!(error.new_handler_name, "handle_movement_MoveStartForward");
    let registry = builder.build();
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::MoveStartForward)
            .count(),
        1,
        "the rejected duplicate must not replace the original entry"
    );
}

/// The trainer family (#1263 F5 tail, commit 1) left the legacy inventory.
#[test]
fn trainer_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "trainer",
        |builder| {
            wow_world_application::register_trainer_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("trainer registrar registers");
        },
        &[
            contract_row(
                0x34AD,
                "TrainerList",
                "handle_trainer_list",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x34AE,
                "TrainerBuySpell",
                "handle_trainer_buy_spell",
                "LoggedIn",
                "Inplace",
            ),
        ],
        25,
    );
}
/// The movement registration tail (#1263 F5 tail, commit 2) left the legacy
/// inventory: 53 macro-origin entries plus the one direct `MoveSplineDone`
/// entry, all from its own area registrar.
#[test]
fn movement_tail_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "movement tail",
        |builder| {
            wow_world_application::register_movement_tail_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("movement tail registrar registers");
        },
        &[
            contract_row(
                0x39E4,
                "MoveStartForward",
                "handle_movement_MoveStartForward",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39E5,
                "MoveStartBackward",
                "handle_movement_MoveStartBackward",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39E6,
                "MoveStop",
                "handle_movement_MoveStop",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39E7,
                "MoveStartStrafeLeft",
                "handle_movement_MoveStartStrafeLeft",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39E8,
                "MoveStartStrafeRight",
                "handle_movement_MoveStartStrafeRight",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39E9,
                "MoveStopStrafe",
                "handle_movement_MoveStopStrafe",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39EA,
                "MoveJump",
                "handle_movement_MoveJump",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39EC,
                "MoveStartTurnLeft",
                "handle_movement_MoveStartTurnLeft",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39ED,
                "MoveStartTurnRight",
                "handle_movement_MoveStartTurnRight",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39EE,
                "MoveStopTurn",
                "handle_movement_MoveStopTurn",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39EF,
                "MoveStartPitchUp",
                "handle_movement_MoveStartPitchUp",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39F0,
                "MoveStartPitchDown",
                "handle_movement_MoveStartPitchDown",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39F1,
                "MoveStopPitch",
                "handle_movement_MoveStopPitch",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39F2,
                "MoveSetRunMode",
                "handle_movement_MoveSetRunMode",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39F3,
                "MoveSetWalkMode",
                "handle_movement_MoveSetWalkMode",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39FB,
                "MoveFallLand",
                "handle_movement_MoveFallLand",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39FC,
                "MoveStartSwim",
                "handle_movement_MoveStartSwim",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x39FD,
                "MoveStopSwim",
                "handle_movement_MoveStopSwim",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A09,
                "MoveSetFacing",
                "handle_movement_MoveSetFacing",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0A,
                "MoveSetPitch",
                "handle_movement_MoveSetPitch",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0B,
                "MoveForceRunSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0C,
                "MoveForceRunBackSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0D,
                "MoveForceSwimSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0E,
                "MoveForceRootAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A0F,
                "MoveForceUnrootAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A10,
                "MoveHeartbeat",
                "handle_movement_MoveHeartbeat",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A13,
                "MoveHoverAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A18,
                "MoveSplineDone",
                "handle_move_spline_done",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A19,
                "MoveFallReset",
                "handle_movement_MoveFallReset",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A1A,
                "MoveUpdateFallSpeed",
                "handle_movement_MoveUpdateFallSpeed",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A1C,
                "MoveFeatherFallAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A1D,
                "MoveWaterWalkAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A1E,
                "MoveEnableDoubleJumpAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A21,
                "MoveForceWalkSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A22,
                "MoveForceSwimBackSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A23,
                "MoveForceTurnRateChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A24,
                "MoveEnableSwimToFlyTransAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A25,
                "MoveSetCanTurnWhileFallingAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A26,
                "MoveSetIgnoreMovementForcesAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A27,
                "MoveSetCanFlyAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A28,
                "MoveSetFly",
                "handle_movement_MoveSetFly",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A29,
                "MoveStartAscend",
                "handle_movement_MoveStartAscend",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A2A,
                "MoveStopAscend",
                "handle_movement_MoveStopAscend",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A2D,
                "MoveForceFlightSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A2E,
                "MoveForceFlightBackSpeedChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A30,
                "MoveStartDescend",
                "handle_movement_MoveStartDescend",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A32,
                "MoveForcePitchRateChangeAck",
                "handle_movement_speed_ack",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A35,
                "MoveGravityDisableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A36,
                "MoveGravityEnableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A37,
                "MoveInertiaDisableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A38,
                "MoveInertiaEnableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A39,
                "MoveCollisionDisableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A3A,
                "MoveCollisionEnableAck",
                "handle_movement_ack_message",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x3A5F,
                "MoveSetFacingHeartbeat",
                "handle_movement_MoveSetFacingHeartbeat",
                "LoggedIn",
                "ThreadSafe",
            ),
        ],
        25,
    );
}

/// The character/account registration family (#1263 F5 remaining families) left
/// the legacy inventory: 23 entries in the four files that lived under
/// `handlers/character/account/registrations/`, all from its own area registrar.
#[test]
fn character_account_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "character/account",
        |builder| {
            wow_world_application::register_character_account_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("character/account registrar registers");
        },
        &[
            contract_row(
                0x34A1,
                "ListInventory",
                "handle_list_inventory",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(0x34A3, "BuyItem", "handle_buy_item", "LoggedIn", "Inplace"),
            contract_row(
                0x34A4,
                "BuyBackItem",
                "handle_buy_back_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x34A2,
                "SellItem",
                "handle_sell_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3530,
                "ItemPurchaseRefund",
                "handle_item_purchase_refund",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x34B3,
                "BankerActivate",
                "handle_banker_activate",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3997,
                "AutobankItem",
                "handle_autobank_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3996,
                "AutostoreBankItem",
                "handle_autostore_bank_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x34B4,
                "BuyBankSlot",
                "handle_buy_bank_slot",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x34B2,
                "BinderActivate",
                "handle_binder_activate",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3506,
                "HearthAndResurrect",
                "handle_hearth_and_resurrect",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x34EC,
                "RepairItem",
                "handle_repair_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x349D,
                "QuestGiverStatusMultipleQuery",
                "handle_quest_giver_status_multiple_query",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x356B,
                "QuestGiverStatusTrackedQuery",
                "handle_quest_giver_status_tracked_query",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x399B,
                "SwapInvItem",
                "handle_swap_inv_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3998,
                "AutoEquipItem",
                "handle_auto_equip_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x399D,
                "AutoEquipItemSlot",
                "handle_auto_equip_item_slot",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x399A,
                "SwapItem",
                "handle_swap_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3999,
                "AutoStoreBagItem",
                "handle_auto_store_bag_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3293,
                "DestroyItem",
                "handle_destroy_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3492,
                "TalkToGossip",
                "handle_gossip_hello",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3494,
                "GossipSelectOption",
                "handle_gossip_select_option",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x34D6,
                "LogoutRequest",
                "handle_logout_request",
                "LoggedIn",
                "ThreadUnsafe",
            ),
        ],
        25,
    );
}

/// Negative control (#1263 F5 remaining families): re-registering the migrated
/// character/account family on the same builder is rejected and never replaces
/// the first entry.
#[test]
fn duplicated_migrated_character_account_registration_is_rejected_without_replacement() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_character_account_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect("the first character/account registration succeeds");
    let error = wow_world_application::register_character_account_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect_err("a second character/account registration must be rejected");
    assert_eq!(error.opcode, wow_constants::ClientOpcodes::ListInventory);
    assert_eq!(error.previous_handler_name, "handle_list_inventory");
    assert_eq!(error.new_handler_name, "handle_list_inventory");
    let registry = builder.build();
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::ListInventory)
            .count(),
        1,
        "the rejected duplicate must not replace the original entry"
    );
}

/// The quest-giver interaction registration family (#1263 F5 remaining
/// families) left the legacy inventory: the eleven entries that lived in
/// `handlers/quest/handlers.rs`, all from its own area registrar.
#[test]
fn quest_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "quest",
        |builder| {
            wow_world_application::register_quest_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("quest registrar registers");
        },
        &[
            contract_row(
                0x32D9,
                "AdventureMapStartQuest",
                "handle_adventure_map_start_quest",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x349C,
                "QuestGiverStatusQuery",
                "handle_quest_giver_status_query",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3496,
                "QuestGiverHello",
                "handle_quest_giver_hello",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3497,
                "QuestGiverQueryQuest",
                "handle_quest_giver_query_quest",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3498,
                "QuestGiverAcceptQuest",
                "handle_quest_giver_accept_quest",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x36B2,
                "QuestPoiQuery",
                "handle_quest_poi_query",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x349B,
                "QuestGiverRequestReward",
                "handle_quest_giver_request_reward",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3499,
                "QuestGiverCompleteQuest",
                "handle_quest_giver_complete_quest",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x349A,
                "QuestGiverChooseReward",
                "handle_quest_giver_choose_reward",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x349E,
                "QuestConfirmAccept",
                "handle_quest_confirm_accept",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x349F,
                "PushQuestToParty",
                "handle_push_quest_to_party",
                "LoggedIn",
                "ThreadUnsafe",
            ),
        ],
        25,
    );
}

/// The spell cast/open registration family (#1263 F5 remaining families) left
/// the legacy inventory: the four non-cancellation `SpellHandler.cpp` entries
/// that lived in `handlers/spell.rs`, all from the already-owned spell area
/// registrar.
#[test]
fn spell_cast_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "spell cast/open",
        |builder| {
            wow_world_application::register_spell_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("spell registrar registers");
        },
        &[
            contract_row(
                0x329C,
                "CastSpell",
                "handle_cast_spell",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x32C6,
                "OpenItem",
                "handle_open_item",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x3531,
                "SelfRes",
                "handle_self_res",
                "LoggedIn",
                "ThreadUnsafe",
            ),
            contract_row(
                0x3495,
                "SpellClick",
                "handle_spell_click",
                "LoggedIn",
                "Inplace",
            ),
        ],
        25,
    );
}

/// The travel/transfer registration family (#1263 F5 remaining families) left
/// the legacy inventory: the three entries that lived in
/// `handlers/travel/travel.rs`, all from the already-owned travel area
/// registrar.
#[test]
fn travel_family_is_migrated_off_the_legacy_inventory() {
    assert_migrated_family_like_cpp(
        "travel",
        |builder| {
            wow_world_application::register_travel_handlers_like_cpp::<
                crate::session::WorldSession,
                crate::session::SessionHandlerCatalogsLikeCpp,
            >(builder)
            .expect("travel registrar registers");
        },
        &[
            contract_row(
                0x34AB,
                "ActivateTaxi",
                "handle_activate_taxi",
                "LoggedIn",
                "ThreadSafe",
            ),
            contract_row(
                0x31D6,
                "AreaTrigger",
                "handle_area_trigger",
                "LoggedIn",
                "Inplace",
            ),
            contract_row(
                0x35FA,
                "WorldPortResponse",
                "handle_world_port_response",
                "Transfer",
                "ThreadUnsafe",
            ),
        ],
        25,
    );
}

/// Negative control (#1263 F5 remaining families): re-registering the migrated
/// quest family on the same builder is rejected and never replaces the first
/// entry.
#[test]
fn duplicated_migrated_quest_registration_is_rejected_without_replacement() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_quest_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect("the first quest registration succeeds");
    let error = wow_world_application::register_quest_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect_err("a second quest registration must be rejected");
    assert_eq!(
        error.opcode,
        wow_constants::ClientOpcodes::AdventureMapStartQuest
    );
    assert_eq!(
        error.previous_handler_name,
        "handle_adventure_map_start_quest"
    );
    assert_eq!(error.new_handler_name, "handle_adventure_map_start_quest");
    let registry = builder.build();
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::AdventureMapStartQuest)
            .count(),
        1,
        "the rejected duplicate must not replace the original entry"
    );
}

/// Negative control (#1263 F5 remaining families): re-registering the extended
/// spell family on the same builder is rejected and never replaces the first
/// entry.
#[test]
fn duplicated_migrated_spell_family_registration_is_rejected_without_replacement() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_spell_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect("the first spell registration succeeds");
    let error = wow_world_application::register_spell_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect_err("a second spell registration must be rejected");
    assert_eq!(error.opcode, wow_constants::ClientOpcodes::CancelCast);
    assert_eq!(error.previous_handler_name, "handle_cancel_cast");
    assert_eq!(error.new_handler_name, "handle_cancel_cast");
    let registry = builder.build();
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::CastSpell)
            .count(),
        1,
        "the rejected duplicate must not replace the cast-spell entry"
    );
}

/// Negative control (#1263 F5 remaining families): re-registering the extended
/// travel family on the same builder is rejected and never replaces the first
/// entry.
#[test]
fn duplicated_migrated_travel_family_registration_is_rejected_without_replacement() {
    let mut builder = crate::session::registry::WorldPacketHandlerRegistryBuilder::new();
    wow_world_application::register_travel_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect("the first travel registration succeeds");
    let error = wow_world_application::register_travel_handlers_like_cpp::<
        crate::session::WorldSession,
        crate::session::SessionHandlerCatalogsLikeCpp,
    >(&mut builder)
    .expect_err("a second travel registration must be rejected");
    assert_eq!(
        error.opcode,
        wow_constants::ClientOpcodes::SuspendTokenResponse
    );
    assert_eq!(error.previous_handler_name, "handle_suspend_token_response");
    assert_eq!(error.new_handler_name, "handle_suspend_token_response");
    let registry = builder.build();
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.opcode == wow_constants::ClientOpcodes::ActivateTaxi)
            .count(),
        1,
        "the rejected duplicate must not replace the activate-taxi entry"
    );
}
