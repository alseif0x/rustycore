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
    InspectHonorStatsResponse, InspectItem, InspectResult, QueryInspectAchievements,
    RequestHonorStats, RespondInspectAchievements,
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryInspectAchievements,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_inspect_achievements",
    }
}

// ── handler implementation ────────────────────────────────────────────────────

impl WorldSession {
    const INSPECT_DISTANCE_LIKE_CPP: f32 = 28.0;

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

    /// CMSG_QUERY_INSPECT_ACHIEVEMENTS (0x3500)
    ///
    /// C++ `HandleQueryInspectAchievements` returns silently if the target is
    /// missing, out of inspect range (`INSPECT_DISTANCE`, 2D), or a valid
    /// attack target. Rust currently has no `PlayerAchievementMgr`, so the
    /// success branch sends a structurally correct empty
    /// `SMSG_RESPOND_INSPECT_ACHIEVEMENTS`.
    pub async fn handle_query_inspect_achievements(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match QueryInspectAchievements::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("QueryInspectAchievements: failed to read target: {}", e);
                return;
            }
        };

        let registry = match self.player_registry() {
            Some(r) => r.clone(),
            None => return,
        };

        let target = match registry.get(&request.guid) {
            Some(e) => e.value().clone(),
            None => return,
        };

        let self_position = match self.player_position_like_cpp() {
            Some(position) => position,
            None => return,
        };

        if target.map_id != self.player_map_id_like_cpp()
            || !target
                .position
                .is_within_dist_2d(&self_position, Self::INSPECT_DISTANCE_LIKE_CPP)
        {
            return;
        }

        // Conservative represented `IsValidAttackTarget` guard: without the
        // full faction/PvP combat targetability graph, reject clearly different
        // non-zero faction-template pairs rather than leaking inspect data.
        let self_faction = self.player_faction_template_id_like_cpp().unwrap_or(0);
        if self_faction != 0
            && target.faction_template_id != 0
            && self_faction != target.faction_template_id
        {
            return;
        }

        self.send_packet(&RespondInspectAchievements {
            player: request.guid,
        });
    }
}
