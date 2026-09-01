// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private player capability handlers extracted from the legacy misc owner.

use tracing::{debug, info, warn};
use wow_constants::{ClientOpcodes, UnitStandStateType};
use wow_core::{GameTime, ObjectGuid};
use wow_handler::{PacketProcessing, SessionStatus};
use wow_persistence::{
    RepresentedGroupPersistenceModeLikeCpp, RepresentedGroupPersistenceOutcomeLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp,
};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::character::SetTitle;
use wow_packet::packets::item::{GetItemPurchaseData, SetItemPurchaseData};
use wow_packet::packets::misc::{
    FarSight, MailNextTimeEntry, MailQueryNextTimeResult, SetDifficultyId, SetDungeonDifficulty,
    SetRaidDifficulty, StandStateChange, ToggleDifficulty,
};
use wow_packet::packets::spell::SetActionButton;

use super::{RepresentedInstanceResetMethodLikeCpp, item_purchase_contents_from_extended_cost};
use crate::entity_update_bridge::player_values_update_to_update_object;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::FarSight,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_far_sight",
        handler: |session, pkt| Box::pin(async move { session.handle_far_sight(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetSelection,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_selection",
        handler: |session, pkt| Box::pin(async move { session.handle_set_selection(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::StandStateChange,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_stand_state_change",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_stand_state_change(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_time",
        handler: |session, _pkt| Box::pin(async move { session.handle_query_time().await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryNextMailTime,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_next_mail_time",
        handler: |session, _pkt| {
            Box::pin(async move { session.handle_query_next_mail_time().await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetActionButton,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_action_button",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_action_button(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetDifficultyId,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_difficulty_id",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_difficulty_id(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ToggleDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_toggle_difficulty",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_toggle_difficulty(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetDungeonDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_dungeon_difficulty",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_dungeon_difficulty(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetRaidDifficulty,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_raid_difficulty",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_set_raid_difficulty(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetTitle,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_title",
        handler: |session, pkt| Box::pin(async move { session.handle_set_title(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::GetItemPurchaseData,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_get_item_purchase_data",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_get_item_purchase_data(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    /// C++ `WorldSession::HandleFarSightOpcode`: does not create/remove the
    /// viewpoint; it only switches the represented seer and forces visibility.
    pub async fn handle_far_sight(&mut self, mut pkt: wow_packet::WorldPacket) {
        let far_sight = match FarSight::read(&mut pkt) {
            Ok(far_sight) => far_sight,
            Err(err) => {
                warn!("Failed to read FarSight: {err}");
                return;
            }
        };

        self.apply_far_sight_like_cpp(far_sight.enable);
        self.force_update_visibility_like_cpp().await;
    }

    /// CMSG_SET_SELECTION — client clicked/targeted an object.
    /// Payload: packed GUID of selected object (0 clears selection).

    pub async fn handle_set_selection(&mut self, mut pkt: wow_packet::WorldPacket) {
        let target_guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        self.set_selection_guid_like_cpp(Some(target_guid));
        info!(
            "SetSelection: account {} → {:?}",
            self.account_id, target_guid
        );
    }

    pub async fn handle_stand_state_change(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match StandStateChange::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    // ── QueryTime ─────────────────────────────────────────────────────────────

    /// CMSG_QUERY_TIME — client requests current server time.
    /// C# ref: QueryHandler.HandleQueryTime → SendQueryTimeResponse
    pub async fn handle_query_time(&mut self) {
        use std::time::{SystemTime, UNIX_EPOCH};
        use wow_packet::packets::misc::QueryTimeResponse;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        self.send_packet(&QueryTimeResponse { current_time: ts });
    }

    // ── QueryNextMailTime ──────────────────────────────────────────────────────

    pub async fn handle_query_next_mail_time(&mut self) {
        const MAIL_CHECK_MASK_READ_LIKE_CPP: u8 = 0x01;
        const MAIL_NORMAL_LIKE_CPP: u8 = 0;

        let Some(port) = self.next_mail_time_persistence_port_like_cpp() else {
            self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
            return;
        };

        let Some(player_object_guid) = self.player_guid() else {
            self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
            return;
        };

        let player_guid = player_object_guid.counter() as u64;
        let now = GameTime::now().as_secs() as i64;
        let rows = match port
            .load_next_mail_time_rows_like_cpp(wow_persistence::NextMailTimeLoadRequestLikeCpp {
                player_guid,
            })
            .await
        {
            wow_persistence::NextMailTimeLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::NextMailTimeLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    error = %reason,
                    player_guid, "Failed to query mail for CMSG_QUERY_NEXT_MAIL_TIME"
                );
                self.send_packet_realm(&MailQueryNextTimeResult::no_mail());
                return;
            }
        };

        let mut packet = MailQueryNextTimeResult::no_mail();
        let mut sent_senders = std::collections::BTreeSet::new();

        for row in rows {
            if (row.checked & MAIL_CHECK_MASK_READ_LIKE_CPP) == 0
                && now >= row.deliver_time
                && sent_senders.insert(row.sender)
            {
                let sender_guid = if row.message_type == MAIL_NORMAL_LIKE_CPP {
                    ObjectGuid::create_player(self.realm_id(), row.sender as i64)
                } else {
                    ObjectGuid::EMPTY
                };

                packet.next_mail_time = 0.0;
                packet.next.push(MailNextTimeEntry {
                    sender_guid,
                    time_left: (row.deliver_time - now) as f32,
                    alt_sender_id: if row.message_type == MAIL_NORMAL_LIKE_CPP {
                        0
                    } else {
                        row.sender as i32
                    },
                    alt_sender_type: row.message_type as i8,
                    stationery_id: row.stationery,
                });

                if sent_senders.len() > 2 {
                    break;
                }
            }
        }

        self.send_packet_realm(&packet);
    }

    pub async fn handle_set_action_button(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetActionButton::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetActionButton parse failed: {error}"
                );
                return;
            }
        };

        self.represented_set_action_button_like_cpp(packet.index, packet.action);
    }

    pub async fn handle_set_difficulty_id(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetDifficultyId::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetDifficultyId parse failed: {error}"
                );
                return;
            }
        };

        self.apply_represented_difficulty_change_like_cpp(packet.difficulty_id)
            .await;
    }

    pub async fn handle_toggle_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = ToggleDifficulty::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "ToggleDifficulty parse failed: {error}"
            );
            return;
        }

        let Some(difficulty_id) = self.represented_toggle_difficulty_target_like_cpp() else {
            debug!(
                account = self.account_id,
                "ToggleDifficulty has no represented toggle difficulty available"
            );
            return;
        };

        self.apply_represented_difficulty_change_like_cpp(difficulty_id)
            .await;
    }

    pub async fn handle_set_dungeon_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetDungeonDifficulty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetDungeonDifficulty parse failed: {error}"
                );
                return;
            }
        };

        self.apply_represented_difficulty_change_like_cpp(packet.difficulty_id)
            .await;
    }

    pub async fn handle_set_raid_difficulty(&mut self, mut pkt: wow_packet::WorldPacket) {
        let packet = match SetRaidDifficulty::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SetRaidDifficulty parse failed: {error}"
                );
                return;
            }
        };

        let Some(difficulty_id) = self
            .represented_raid_difficulty_request_like_cpp(packet.difficulty_id, packet.legacy != 0)
        else {
            return;
        };

        self.apply_represented_difficulty_change_like_cpp(difficulty_id)
            .await;
    }

    async fn apply_represented_difficulty_change_like_cpp(&mut self, difficulty_id: u32) {
        let reset_owner = self.represented_set_difficulty_reset_owner_like_cpp(difficulty_id);
        if let Some(reset_owner) = reset_owner {
            self.reset_represented_instances_like_cpp(
                reset_owner,
                RepresentedInstanceResetMethodLikeCpp::OnChangeDifficulty,
            )
            .await;
        }

        let commands = self.represented_set_difficulty_id_like_cpp(difficulty_id);
        if commands.is_empty() {
            return;
        }

        let Some(port) = self.represented_group_persistence_port_like_cpp() else {
            return;
        };
        let outcome = port
            .persist_group_commands_like_cpp(RepresentedGroupPersistenceRequestLikeCpp {
                commands,
                mode: RepresentedGroupPersistenceModeLikeCpp::Atomic,
            })
            .await;
        if !matches!(
            outcome,
            RepresentedGroupPersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                ?outcome,
                "failed to persist represented group difficulty change"
            );
        }
    }

    pub async fn handle_set_title(&mut self, mut pkt: wow_packet::WorldPacket) {
        let mut packet = match SetTitle::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(account = self.account_id, "SetTitle parse failed: {error}");
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
        if let Some(update) = self.set_canonical_chosen_title_like_cpp(packet.title_id) {
            if let Some(player_guid) = self.player_guid() {
                if let Some(packet) = player_values_update_to_update_object(
                    player_guid,
                    self.player_map_id_like_cpp(),
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
        let current_total_played_time = self.total_played_time.saturating_add(
            self.login_time
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
