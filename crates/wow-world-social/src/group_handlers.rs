// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group and party handler packet bodies.
//!
//! C++ source of truth: `GroupHandler.cpp` for the leader/assistant, role,
//! raid-target, ready-check, minimap-ping and random-roll opcodes. The family
//! owns the packet bodies and the group-registry transitions; the World session
//! only builds the borrowed social state plus hub context (#1263 F5). The
//! invite/leave/convert/subgroup handlers and their lifecycle persistence stay
//! in the World shell for a later slice.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::party::{
    DoReadyCheck, InitiateRolePoll, LowLevelRaid1, LowLevelRaid2, MinimapPing, MinimapPingClient,
    ReadyCheckResponseClient, RequestPartyJoinUpdates, RequestPartyMemberStats, RoleChangedInform,
    SetLootMethod, SetRole, SilencePartyTalker, UpdateRaidTarget,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

use crate::SessionSocialLimits;
use crate::group_fanout::{
    connected_group_member_txs_like_cpp, connected_group_members_like_cpp,
    current_group_guid_like_cpp, party_member_full_state_like_cpp, raid_markers_changed_like_cpp,
    raid_target_update_all_like_cpp, raid_target_update_single_like_cpp,
    role_changed_inform_like_cpp, role_poll_inform_like_cpp, send_group_packet_bytes_like_cpp,
    send_party_update, send_ready_check_events_like_cpp, sender_can_start_ready_check_like_cpp,
};

/// Borrowed inputs of one group handler invocation.
pub struct SocialGroupHandlerCxLikeCpp<'a> {
    social: &'a mut SessionSocialLimits,
    hub: HubMut<'a>,
}

impl<'a> SocialGroupHandlerCxLikeCpp<'a> {
    pub fn new(social: &'a mut SessionSocialLimits, hub: HubMut<'a>) -> Self {
        Self { social, hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let owner = self.hub.shared().core.player_group_owner_access_like_cpp();
        self.social.resolved_group_guid_with_access_like_cpp(
            &owner,
            cfg!(any(test, feature = "test-fixtures")),
        )
    }

    pub async fn handle_set_loot_method(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = SetLootMethod::read(&mut pkt) {
            warn!("Bad SetLootMethod: {e}");
        }
    }

    pub async fn handle_silence_party_talker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let silence = match SilencePartyTalker::read(&mut pkt) {
            Ok(silence) => silence,
            Err(e) => {
                warn!("Bad SilencePartyTalker: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            None,
        ) else {
            return;
        };
        let Some(group) = group_reg.get(&group_guid) else {
            return;
        };
        if !group.is_leader_like_cpp(sender_guid) && !group.is_assistant_like_cpp(sender_guid) {
            return;
        }

        self.social
            .record_represented_silence_party_talker_like_cpp(silence.target, silence.silent);
    }

    pub async fn handle_set_role(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_role = match SetRole::read(&mut pkt) {
            Ok(set_role) => set_role,
            Err(e) => {
                warn!("Bad SetRole: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => {
                if set_role.role == 0 {
                    return;
                }
                self.publication_like_cpp().send_packet(&RoleChangedInform {
                    party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                    from: sender_guid,
                    changed_unit: set_role.target_guid,
                    old_role: 0,
                    new_role: set_role.role,
                });
                return;
            }
        };

        let group_guid = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            set_role.party_index,
        );

        let Some(group_guid) = group_guid else {
            if set_role.role == 0 {
                return;
            }
            self.publication_like_cpp().send_packet(&RoleChangedInform {
                party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                from: sender_guid,
                changed_unit: set_role.target_guid,
                old_role: 0,
                new_role: set_role.role,
            });
            return;
        };

        let registry = self
            .hub
            .shared()
            .core
            .player_registry()
            .map(std::sync::Arc::clone);
        let outcome = match group_reg.set_lfg_role_transition_like_cpp(
            group_guid,
            set_role.target_guid,
            set_role.role,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let (old_role, lfg_roles_mutated_existing_target) = outcome.facts;
        let recipients = registry
            .as_ref()
            .map(|registry| connected_group_member_txs_like_cpp(&outcome.group, registry))
            .unwrap_or_default();
        let bytes = role_changed_inform_like_cpp(
            outcome.group.group_category_like_cpp(),
            sender_guid,
            set_role.target_guid,
            old_role,
            set_role.role,
        );

        // C++ broadcasts RoleChangedInform, then Group::SetLfgRoles mutates an
        // existing member slot and calls SendUpdate(). Keep both fanouts outside
        // the mutable guard and only send PartyUpdate when the slot existed.
        if let Some(registry) = registry.as_ref() {
            send_group_packet_bytes_like_cpp(registry, bytes, &recipients);
        }

        if lfg_roles_mutated_existing_target {
            if let Some(registry) = registry.as_ref() {
                let vra = self.hub.shared().core.virtual_realm_address();
                send_party_update(&outcome.group, registry, vra);
            }
        }
    }

    pub async fn handle_initiate_role_poll(&mut self, mut pkt: wow_packet::WorldPacket) {
        let role_poll = match InitiateRolePoll::read(&mut pkt) {
            Ok(role_poll) => role_poll,
            Err(e) => {
                warn!("Bad InitiateRolePoll: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.hub.shared().core.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            role_poll.party_index,
        ) else {
            return;
        };

        let Some((bytes, recipients)) = group_reg.get(&group_guid).and_then(|group| {
            if !sender_can_start_ready_check_like_cpp(&group, sender_guid) {
                return None;
            }
            Some((
                role_poll_inform_like_cpp(group.group_category_like_cpp() as i8, sender_guid),
                connected_group_member_txs_like_cpp(&group, &registry),
            ))
        }) else {
            return;
        };

        send_group_packet_bytes_like_cpp(&registry, bytes, &recipients);
    }

    pub async fn handle_update_raid_target(&mut self, mut pkt: wow_packet::WorldPacket) {
        let update = match UpdateRaidTarget::read(&mut pkt) {
            Ok(update) => update,
            Err(e) => {
                warn!("Bad UpdateRaidTarget: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            update.party_index,
        ) else {
            return;
        };

        if update.symbol == -1 {
            if let Some(group) = group_reg.get(&group_guid) {
                self.hub
                    .shared()
                    .core
                    .send_raw_packet(&raid_target_update_all_like_cpp(&group));
            }
            return;
        }

        let Ok(symbol) = u8::try_from(update.symbol) else {
            return;
        };
        let registry = match self.hub.shared().core.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        if update.target.is_player()
            && !update.target.is_empty()
            && registry.group_presence(update.target).is_none()
        {
            return;
        }

        let outcome = match group_reg.set_target_icon_transition_like_cpp(
            group_guid,
            sender_guid,
            symbol,
            update.target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let recipients = connected_group_member_txs_like_cpp(&outcome.group, &registry);
        let party_index = outcome.group.group_category_like_cpp();

        for (changed_symbol, target) in outcome.facts {
            send_group_packet_bytes_like_cpp(
                &registry,
                raid_target_update_single_like_cpp(
                    party_index,
                    changed_symbol,
                    target,
                    sender_guid,
                ),
                &recipients,
            );
        }
    }

    pub async fn handle_request_party_join_updates(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyJoinUpdates::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyJoinUpdates: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            request.party_index,
        ) else {
            return;
        };
        if let Some(group) = group_reg.get(&group_guid) {
            self.hub
                .shared()
                .core
                .send_raw_packet(&raid_target_update_all_like_cpp(&group));
            self.hub
                .shared()
                .core
                .send_raw_packet(&raid_markers_changed_like_cpp(&group));
        }
    }

    pub async fn handle_request_party_member_stats(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyMemberStats::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyMemberStats: {e}");
                return;
            }
        };

        let registry = self
            .hub
            .shared()
            .core
            .player_registry()
            .map(std::sync::Arc::clone);
        let state = party_member_full_state_like_cpp(request.target_guid, registry.as_deref());
        self.publication_like_cpp().send_packet_realm(&state);
    }

    pub async fn handle_do_ready_check(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ready_check = match DoReadyCheck::read(&mut pkt) {
            Ok(ready_check) => ready_check,
            Err(e) => {
                warn!("Bad DoReadyCheck: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.hub.shared().core.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            ready_check.party_index,
        ) else {
            return;
        };

        let connected = group_reg
            .get(&group_guid)
            .map(|group| connected_group_members_like_cpp(&group, &registry))
            .unwrap_or_default();
        let outcome = match group_reg.start_ready_check_transition_like_cpp(
            group_guid,
            sender_guid,
            connected,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        send_ready_check_events_like_cpp(&outcome.facts, &outcome.group, &registry);
    }

    pub async fn handle_ready_check_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let response = match ReadyCheckResponseClient::read(&mut pkt) {
            Ok(response) => response,
            Err(e) => {
                warn!("Bad ReadyCheckResponse: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.hub.shared().core.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            response.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.respond_ready_check_transition_like_cpp(
            group_guid,
            sender_guid,
            response.is_ready,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        send_ready_check_events_like_cpp(&outcome.facts, &outcome.group, &registry);
    }

    pub async fn handle_low_level_raid1(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid1::read(&mut pkt) {
            warn!("Bad LowLevelRaid1: {e}");
            return;
        }
        if let Some(guid) = self.hub.shared().core.player_guid() {
            tracing::debug!("HandleLowLevelRaid1 - Player {:?}", guid);
        }
    }

    pub async fn handle_low_level_raid2(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid2::read(&mut pkt) {
            warn!("Bad LowLevelRaid2: {e}");
            return;
        }
        if let Some(guid) = self.hub.shared().core.player_guid() {
            tracing::debug!("HandleLowLevelRaid2 - Player {:?}", guid);
        }
    }

    pub async fn handle_minimap_ping(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ping = match MinimapPingClient::read(&mut pkt) {
            Ok(ping) => ping,
            Err(e) => {
                warn!("Bad MinimapPing: {e}");
                return;
            }
        };
        let sender_guid = match self.hub.shared().core.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.hub.shared().core.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.hub.shared().core.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            ping.party_index,
        ) else {
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            return;
        };

        let bytes = MinimapPing {
            sender: sender_guid,
            position_x: ping.position_x,
            position_y: ping.position_y,
        }
        .to_bytes();

        // C++ BroadcastPacket(packet, true, -1, GetPlayer()->GetGUID()) excludes sender.
        for member_guid in &group.members {
            if *member_guid == sender_guid {
                continue;
            }
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
            }
        }
    }
}

/// Builds a group handler context from a host's social state and hub.
pub trait SocialGroupHandlerHostLikeCpp<C> {
    fn social_group_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> SocialGroupHandlerCxLikeCpp<'a>;
}

fn handle_set_loot_method_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_set_loot_method(pkt)
            .await;
    })
}

fn handle_silence_party_talker_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_silence_party_talker(pkt)
            .await;
    })
}

fn handle_set_role_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_set_role(pkt)
            .await;
    })
}

fn handle_initiate_role_poll_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_initiate_role_poll(pkt)
            .await;
    })
}

fn handle_update_raid_target_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_update_raid_target(pkt)
            .await;
    })
}

fn handle_request_party_join_updates_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_request_party_join_updates(pkt)
            .await;
    })
}

fn handle_request_party_member_stats_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_request_party_member_stats(pkt)
            .await;
    })
}

fn handle_do_ready_check_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_do_ready_check(pkt)
            .await;
    })
}

fn handle_ready_check_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_ready_check_response(pkt)
            .await;
    })
}

fn handle_low_level_raid1_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_low_level_raid1(pkt)
            .await;
    })
}

fn handle_low_level_raid2_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_low_level_raid2(pkt)
            .await;
    })
}

fn handle_minimap_ping_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .social_group_handler_cx_like_cpp(catalogs)
            .handle_minimap_ping(pkt)
            .await;
    })
}

/// Registers the group handlers on the packet registry.
pub fn register_social_group_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: SocialGroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootMethod,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_method",
        handler: handle_set_loot_method_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SilencePartyTalker,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_silence_party_talker",
        handler: handle_silence_party_talker_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetRole,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_role",
        handler: handle_set_role_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::InitiateRolePoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_initiate_role_poll",
        handler: handle_initiate_role_poll_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::UpdateRaidTarget,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_update_raid_target",
        handler: handle_update_raid_target_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyJoinUpdates,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_join_updates",
        handler: handle_request_party_join_updates_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RequestPartyMemberStats,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_party_member_stats",
        handler: handle_request_party_member_stats_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DoReadyCheck,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_do_ready_check",
        handler: handle_do_ready_check_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ReadyCheckResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_ready_check_response",
        handler: handle_ready_check_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid1,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid1",
        handler: handle_low_level_raid1_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LowLevelRaid2,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_low_level_raid2",
        handler: handle_low_level_raid2_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MinimapPing,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_minimap_ping",
        handler: handle_minimap_ping_thunk::<S, C>,
    })?;
    Ok(())
}
