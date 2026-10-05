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
