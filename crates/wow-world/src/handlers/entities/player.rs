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
        opcode: ClientOpcodes::FarSight,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_far_sight",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_far_sight_with_catalogs_like_cpp(
                        catalogs.creature_spawns.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

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

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTitle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_title",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_set_title(pkt).await }),
    }
}

crate::session::registry::register_packet_handler_like_cpp! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetItemPurchaseData,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_get_item_purchase_data",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_get_item_purchase_data(pkt).await })
        },
    }
}

#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    /// C++ `WorldSession::HandleFarSightOpcode`: does not create/remove the
    /// viewpoint; it only switches the represented seer and forces visibility.
    pub async fn handle_far_sight_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let far_sight = match FarSight::read(&mut pkt) {
            Ok(far_sight) => far_sight,
            Err(err) => {
                warn!("Failed to read FarSight: {err}");
                return;
            }
        };

        self.apply_far_sight_like_cpp(far_sight.enable);
        self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
            .await;
    }

    #[cfg(test)]
    pub async fn handle_far_sight(&mut self, pkt: wow_packet::WorldPacket) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_far_sight_with_catalogs_like_cpp(&catalogs, pkt)
            .await;
    }

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

    pub async fn handle_set_title(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut packet = match SetTitle::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "SetTitle parse failed: {error}"
                );
                return;
            }
        };

        if packet.title_id > 0 {
            if !self.represented_has_title_like_cpp(packet.title_id as u32) {
                return;
            }
        } else {
            packet.title_id = 0;
        }

        self.represented_set_chosen_title_like_cpp(packet.title_id);
        if let Some(update) = self
            .core
            .set_canonical_chosen_title_like_cpp(packet.title_id)
        {
            if let Some(player_guid) = self.player_guid() {
                if let Some(packet) = player_values_update_to_update_object(
                    player_guid,
                    self.core.player_map_id_like_cpp(),
                    &update,
                ) {
                    self.send_packet(&packet);
                }
            }
        }
    }

    pub async fn handle_get_item_purchase_data(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match GetItemPurchaseData::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("GetItemPurchaseData parse failed: {e}");
                return;
            }
        };
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let current_total_played_time = self.lifecycle.total_played_time_like_cpp().saturating_add(
            self.lifecycle
                .login_time_like_cpp()
                .map(|login_time| login_time.elapsed().as_secs() as u32)
                .unwrap_or(0),
        );

        let Some(packet) = (|| {
            let item = self
                .resolved_inventory_item_objects_like_cpp()
                .and_then(|items| items.get(&request.item_guid).cloned())?;
            if !item.is_refundable() || item.refund_recipient() != player_guid {
                return None;
            }

            let played_time = item.played_time(i64::from(current_total_played_time));
            if played_time > 2 * 60 * 60 {
                return None;
            }

            let extended_cost = self
                .item_extended_cost_store()
                .and_then(|store| store.get(item.paid_extended_cost()))?;
            let contents =
                item_purchase_contents_from_extended_cost(extended_cost, item.paid_money());
            Some(SetItemPurchaseData {
                item_guid: request.item_guid,
                contents,
                flags: 0,
                purchase_time: current_total_played_time.saturating_sub(played_time),
            })
        })() else {
            debug!(
                "GetItemPurchaseData ignored for non-refundable or unknown item {:?}",
                request.item_guid
            );
            return;
        };

        self.send_packet(&packet);
    }
}
