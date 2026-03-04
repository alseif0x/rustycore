// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handler for CMSG_INSPECT: responds with SMSG_INSPECT_RESULT.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::inspect::{InspectItem, InspectResult};
use wow_packet::ServerPacket;

use crate::session::WorldSession;

// ── inventory registration ────────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::Inspect,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_inspect",
    }
}

// ── handler implementation ────────────────────────────────────────────────────

impl WorldSession {
    /// CMSG_INSPECT (0x3529)
    ///
    /// Parse: packed_guid
    pub async fn handle_inspect(&mut self, mut pkt: wow_packet::WorldPacket) {
        let target_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => {
                warn!("Inspect: failed to read target_guid: {}", e);
                return;
            }
        };

        let registry = match self.player_registry() {
            Some(r) => r.clone(),
            None => return,
        };

        let entry = match registry.get(&target_guid) {
            Some(e) => {
                use wow_network::player_registry::PlayerBroadcastInfo;
                let info: PlayerBroadcastInfo = e.value().clone();
                info
            }
            None => {
                warn!("Inspect: target {:?} not found in registry", target_guid);
                return;
            }
        };

        // Build item list from visible_items: [(item_id, enchant_display, subclass); 19]
        let mut items: Vec<InspectItem> = Vec::new();
        for (slot, (item_id, _, _)) in entry.visible_items.iter().enumerate() {
            if *item_id != 0 {
                items.push(InspectItem {
                    slot: slot as u8,
                    item_id: *item_id,
                });
            }
        }

        let result = InspectResult {
            target_guid,
            target_name: entry.player_name.clone(),
            race: entry.race,
            class_id: entry.class,
            gender: entry.sex,
            level: entry.level as u32,
            items,
        };

        self.send_packet(&result);
    }
}
