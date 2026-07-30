// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Ping,
        status: SessionStatus::Authed,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ping",
    }
}

#[test]
fn dispatch_table_contains_registered() {
    let table = wow_handler::build_dispatch_table();
    assert!(table.contains_key(&ClientOpcodes::Ping));
    let entry = table[&ClientOpcodes::Ping];
    assert_eq!(entry.handler_name, "handle_ping");
    assert_eq!(entry.status, SessionStatus::Authed);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
}

#[test]
fn contains_handler_check() {
    assert!(wow_handler::contains_handler(ClientOpcodes::Ping));
    assert!(!wow_handler::contains_handler(ClientOpcodes::AttackSwing));
}

#[test]
fn get_handler_found() {
    let entry = wow_handler::get_handler(ClientOpcodes::Ping).unwrap();
    assert_eq!(entry.handler_name, "handle_ping");
}

#[test]
fn get_handler_not_found() {
    assert!(wow_handler::get_handler(ClientOpcodes::AttackSwing).is_none());
}
