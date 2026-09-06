// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private corpse capability handlers extracted from the legacy misc owner.

use tracing::{info, warn};
use wow_constants::{ClientOpcodes, ConditionType};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    PortGraveyard, ReclaimCorpse, RepopRequest, RequestCemeteryListResponse, ResurrectResponse,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ResurrectResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_resurrect_response",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_resurrect_response(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RepopRequest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_repop_request",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_repop_request(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ReclaimCorpse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_reclaim_corpse",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_reclaim_corpse(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestCemeteryList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_cemetery_list",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_request_cemetery_list_with_catalog_like_cpp(
                        catalogs.graveyards.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

impl crate::session::WorldSession {
    /// CMSG_REQUEST_CEMETERY_LIST — client asks for graveyards in zone.
    /// C++ ref: `WorldSession::HandleRequestCemeteryList`.
    pub(crate) async fn handle_request_cemetery_list_with_catalog_like_cpp(
        &mut self,
        graveyard_store: &wow_data::GraveyardStore,
        _pkt: wow_packet::WorldPacket,
    ) {
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
            info!(
                account = self.account_id,
                state = ?self.state(),
                "RUST_CEMETERY_TRACE handler entry"
            );
        }
        let Some((zone_id, area_id)) = self.player_zone_area_like_cpp() else {
            return;
        };
        if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
            info!(
                account = self.account_id,
                state = ?self.state(),
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                player = ?self.player_guid(),
                "RUST_CEMETERY_TRACE handler resolved zone_area"
            );
        }
        let Some(graveyards) = graveyard_store.graveyards_for_zone(zone_id) else {
            info!(
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                player = ?self.player_guid(),
                "No graveyards found in CMSG_REQUEST_CEMETERY_LIST"
            );
            return;
        };

        let mut cemetery_ids = Vec::new();
        for graveyard in graveyards {
            if cemetery_ids.len() >= 16 {
                break;
            }
            if self.graveyard_conditions_meet_like_cpp(&graveyard.conditions) {
                cemetery_ids.push(graveyard.safe_loc_id);
            }
        }

        if cemetery_ids.is_empty() {
            info!(
                zone = zone_id,
                area = area_id,
                map_id = self.player_map_id_like_cpp(),
                candidate_count = graveyards.len(),
                player = ?self.player_guid(),
                "No graveyards passed conditions in CMSG_REQUEST_CEMETERY_LIST"
            );
            return;
        }

        info!(
            zone = zone_id,
            area = area_id,
            map_id = self.player_map_id_like_cpp(),
            candidate_count = graveyards.len(),
            accepted_count = cemetery_ids.len(),
            cemetery_ids = ?cemetery_ids,
            player = ?self.player_guid(),
            "Sending C++ RequestCemeteryListResponse"
        );
        self.send_packet(&RequestCemeteryListResponse {
            is_gossip_triggered: false,
            cemetery_ids,
        });
    }

    #[cfg(test)]
    pub async fn handle_request_cemetery_list(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self
            .graveyard_store()
            .cloned()
            .unwrap_or_else(|| std::sync::Arc::new(wow_data::GraveyardStore::default()));
        self.handle_request_cemetery_list_with_catalog_like_cpp(store.as_ref(), pkt)
            .await;
    }

    fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool {
        let Some(conditions) = conditions_ref.upgrade() else {
            return true;
        };
        if conditions.is_empty() {
            return true;
        }

        let Some(condition_store) = self.condition_store().cloned() else {
            warn!("Cemetery condition check failed closed: missing condition store");
            return false;
        };
        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            warn!("Cemetery condition check failed closed: missing player object");
            return false;
        };

        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return false;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let needs_player_condition_context = conditions.iter().any(|condition| {
            condition.reference_id != 0
                || condition.condition_type == ConditionType::PlayerCondition
        });
        let player_condition_store = needs_player_condition_context
            .then(|| self.player_condition_store().cloned())
            .flatten();
        let player_condition_context = needs_player_condition_context
            .then(|| self.represented_player_condition_context_like_cpp())
            .flatten();

        let mut source_info =
            crate::conditions::ConditionSourceInfo::from_targets(Some(&player_object), None, None);
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        if let (Some(store), Some(context)) = (
            player_condition_store.as_ref(),
            player_condition_context.as_ref(),
        ) {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = context.as_context(self) {
                source_info.set_player_condition_context(0, context);
            }
        }

        crate::conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store.as_ref(),
            |condition, source_info| match crate::conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                crate::conditions::ConditionMeetResult::Evaluated(value) => value,
                crate::conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Cemetery condition check failed closed: unsupported {:?}",
                        condition.condition_type
                    );
                    false
                }
            },
        )
    }

    /// CMSG_RESURRECT_RESPONSE — answer to a pending resurrection request.
    /// C++ ref: `WorldSession::HandleResurrectResponse`.

    pub async fn handle_resurrect_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let response = match ResurrectResponse::read(&mut pkt) {
            Ok(response) => response,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ResurrectResponse parse failed: {error}"
                );
                return;
            }
        };

        if self.resolved_player_is_alive_like_cpp() != Some(false) {
            return;
        }

        if response.response != 0 {
            self.clear_represented_resurrection_request_like_cpp();
            return;
        }

        let Some(request) = self
            .take_represented_resurrection_request_if_requested_by_like_cpp(response.resurrecter)
        else {
            return;
        };

        // C++ teleports to resurrection request location before applying the
        // resurrected state. InstanceScript combat-res charges, aura original
        // caster, and SpawnCorpseBones remain represented gaps.
        self.teleport_to(request.map_id, request.position).await;
        if self.pending_teleport_like_cpp().is_some() || self.near_teleport_pending_like_cpp() {
            self.schedule_represented_resurrection_after_teleport_like_cpp(request);
        } else {
            self.apply_represented_resurrection_health_like_cpp(request.health);
        }
    }

    /// CMSG_REPOP_REQUEST — release spirit.
    /// C++ ref: `WorldSession::HandleRepopRequest`.

    pub async fn handle_repop_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _request = match RepopRequest::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RepopRequest parse failed: {error}"
                );
                return;
            }
        };

        if self.resolved_player_is_alive_like_cpp() != Some(false)
            || self.player_has_ghost_flag_like_cpp()
        {
            return;
        }

        // C++ also blocks `SPELL_AURA_PREVENT_RESURRECTION`, handles JUST_DIED
        // promotion through KillPlayer, removes the pet, builds the corpse, and
        // teleports to the graveyard. Rust has only the represented death/ghost
        // seam here; full corpse/graveyard runtime remains open.
        self.set_player_alive_like_cpp(false);
        self.set_player_ghost_flag_like_cpp(true);
        #[cfg(test)]
        {
            self.represented_repop_at_graveyard_count =
                self.represented_repop_at_graveyard_count.saturating_add(1);
        }
    }

    /// CMSG_CLIENT_PORT_GRAVEYARD — manually teleport ghost to graveyard.
    /// C++ ref: `WorldSession::HandlePortGraveyard`.
    pub async fn try_handle_client_port_graveyard_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        if PortGraveyard::read(&mut pkt).is_err() {
            return false;
        }

        if self.resolved_player_is_alive_like_cpp() != Some(false)
            || !self.player_has_ghost_flag_like_cpp()
        {
            return true;
        }

        // C++ calls `Player::RepopAtGraveyard()`. Rust still represents the
        // graveyard selection/teleport runtime as a counter seam shared with
        // release and instance-lock decline paths.
        #[cfg(test)]
        {
            self.represented_repop_at_graveyard_count =
                self.represented_repop_at_graveyard_count.saturating_add(1);
        }
        true
    }

    /// CMSG_RECLAIM_CORPSE — resurrect at corpse.
    /// C++ ref: `WorldSession::HandleReclaimCorpse`.

    pub async fn handle_reclaim_corpse(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _request = match ReclaimCorpse::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "ReclaimCorpse parse failed: {error}"
                );
                return;
            }
        };

        if self.resolved_player_is_alive_like_cpp() != Some(false) {
            return;
        }

        if !self.player_has_ghost_flag_like_cpp() {
            return;
        }

        // C++ checks arena, live corpse existence, reclaim delay, and distance
        // before `ResurrectPlayer(0.5f)` + `SpawnCorpseBones`. Those require the
        // full player-corpse runtime; this represented slice only clears the
        // ghost/dead state when the already-known C++ gates pass.
        self.set_player_ghost_flag_like_cpp(false);
        let restore_percent = if self.player_in_represented_battleground_like_cpp() {
            1.0
        } else {
            0.5
        };
        self.apply_represented_resurrection_percent_like_cpp(restore_percent);
    }
}
