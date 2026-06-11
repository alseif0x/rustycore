// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Handlers for inspect-family client packets.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::inspect::{
    InspectHonorStatsResponse, InspectItem, InspectResult, RequestHonorStats,
};

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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestHonorStats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_honor_stats",
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

    /// CMSG_REQUEST_HONOR_STATS (0x317e)
    ///
    /// C++ `WorldSession::HandleRequestHonorStats` finds the connected target
    /// player and returns `SMSG_INSPECT_HONOR_STATS`; missing targets produce no
    /// response.
    pub async fn handle_request_honor_stats(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestHonorStats::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("RequestHonorStats: failed to read target: {}", e);
                return;
            }
        };

        let registry = match self.player_registry() {
            Some(r) => r.clone(),
            None => return,
        };

        let entry = match registry.get(&request.target) {
            Some(e) => e.value().clone(),
            None => return,
        };

        let response = InspectHonorStatsResponse {
            target: request.target,
            lifetime_hk: entry.lifetime_honorable_kills,
            today_contribution: entry.this_week_contribution,
            yesterday_contribution: entry.yesterday_contribution,
            today_hk: entry.today_honorable_kills,
            yesterday_hk: entry.yesterday_honorable_kills,
            lifetime_max_rank: entry.lifetime_max_rank,
            honor_level: entry.honor_level,
        };

        self.send_packet(&response);
    }
}
