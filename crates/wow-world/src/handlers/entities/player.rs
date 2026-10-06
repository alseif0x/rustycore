// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private player capability handlers extracted from the legacy misc owner.

use tracing::{debug, warn};
use wow_constants::{ClientOpcodes, UnitStandStateType};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::character::SetTitle;
use wow_packet::packets::item::{GetItemPurchaseData, SetItemPurchaseData};
use wow_packet::packets::misc::{FarSight, StandStateChange};
use wow_packet::packets::spell::SetActionButton;

use super::item_purchase_contents_from_extended_cost;
use crate::entity_update_bridge::player_values_update_to_update_object;

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    pub async fn handle_set_difficulty_id(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_application::handle_set_difficulty_id_like_cpp(&mut cx, pkt).await;
    }

    pub async fn handle_toggle_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_application::handle_toggle_difficulty_like_cpp(&mut cx, pkt).await;
    }

    pub async fn handle_set_dungeon_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_application::handle_set_dungeon_difficulty_like_cpp(&mut cx, pkt).await;
    }

    pub async fn handle_set_raid_difficulty(&mut self, pkt: wow_packet::WorldPacket) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        wow_world_application::handle_set_raid_difficulty_like_cpp(&mut cx, pkt).await;
    }
}
