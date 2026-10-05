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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::StandStateChange,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_stand_state_change",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_stand_state_change(pkt).await })
        },
    }
}

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    pub async fn handle_stand_state_change(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match StandStateChange::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "StandStateChange parse failed: {error}"
                );
                return;
            }
        };

        let stand_state = match packet.stand_state {
            state if state == UnitStandStateType::Stand as u32 => UnitStandStateType::Stand,
            state if state == UnitStandStateType::Sit as u32 => UnitStandStateType::Sit,
            state if state == UnitStandStateType::Sleep as u32 => UnitStandStateType::Sleep,
            state if state == UnitStandStateType::Kneel as u32 => UnitStandStateType::Kneel,
            _ => return,
        };

        let _ = self.apply_represented_live_intent_like_cpp(
            crate::session::RepresentedLiveIntentLikeCpp::StandStateChanged(
                crate::session::RepresentedStandStateChangedLikeCpp { state: stand_state },
            ),
        );
    }

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
