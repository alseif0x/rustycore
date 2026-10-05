// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group leadership and assistant handler packet bodies.
//!
//! C++ source of truth: `GroupHandler.cpp` for `HandleSetPartyLeader`,
//! `HandleSetAssistantLeader`, `HandleSetEveryoneIsAssistant` and
//! `HandleSetPartyAssignment`. This is the cross-domain slice of the group
//! family: the transitions read the social group registry and persist their
//! intents through the lifecycle owner, so the World session only builds the
//! borrowed social, lifecycle and hub context (#1263 F5). The invite/leave/
//! convert/subgroup handlers and their registry-sync helper stay in the World
//! shell for later slices.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::party::{
    SetAssistantLeader, SetEveryoneIsAssistant, SetPartyAssignment, SetPartyLeader,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP;
use wow_world_core::session::HubMut;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;
use wow_world_social::group_fanout::{
    current_group_guid_like_cpp, send_group_new_leader_like_cpp, send_party_update,
};

/// Borrowed inputs of one group leadership/assistant handler invocation.
pub struct GroupHandlerCxLikeCpp<'a> {
    social: &'a mut SessionSocialLimits,
    lifecycle: &'a SessionLifecycleState,
    hub: HubMut<'a>,
}

impl<'a> GroupHandlerCxLikeCpp<'a> {
    pub fn new(
        social: &'a mut SessionSocialLimits,
        lifecycle: &'a SessionLifecycleState,
        hub: HubMut<'a>,
    ) -> Self {
        Self {
            social,
            lifecycle,
            hub,
        }
    }

    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        let owner = self.hub.shared().core.player_group_owner_access_like_cpp();
        self.social.resolved_group_guid_with_access_like_cpp(
            &owner,
            cfg!(any(test, feature = "test-fixtures")),
        )
    }

    pub async fn handle_set_party_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_leader = match SetPartyLeader::read(&mut pkt) {
            Ok(set_leader) => set_leader,
            Err(e) => {
                warn!("Bad SetPartyLeader: {e}");
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
        let Some(target) = registry.social_recipient(set_leader.target_guid) else {
            return;
        };
        let target_name = target.player_name;
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            set_leader.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.change_leader_transition_like_cpp(
            group_guid,
            sender_guid,
            set_leader.target_guid,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_group_new_leader_like_cpp(&outcome.group, &registry, &target_name).await;
        send_party_update(&outcome.group, &registry, vra);
    }

    pub async fn handle_set_assistant_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_assistant = match SetAssistantLeader::read(&mut pkt) {
            Ok(set_assistant) => set_assistant,
            Err(e) => {
                warn!("Bad SetAssistantLeader: {e}");
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
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            set_assistant.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_member_flag_transition_like_cpp(
            group_guid,
            sender_guid,
            set_assistant.target,
            set_assistant.apply,
            MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }

    pub async fn handle_set_everyone_is_assistant(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_everyone = match SetEveryoneIsAssistant::read(&mut pkt) {
            Ok(set_everyone) => set_everyone,
            Err(e) => {
                warn!("Bad SetEveryoneIsAssistant: {e}");
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
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            set_everyone.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_everyone_assistant_transition_like_cpp(
            group_guid,
            sender_guid,
            set_everyone.everyone_is_assistant,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }

    pub async fn handle_set_party_assignment(&mut self, mut pkt: wow_packet::WorldPacket) {
        let assignment = match SetPartyAssignment::read(&mut pkt) {
            Ok(assignment) => assignment,
            Err(e) => {
                warn!("Bad SetPartyAssignment: {e}");
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
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            assignment.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.set_party_assignment_transition_like_cpp(
            group_guid,
            sender_guid,
            assignment.target,
            assignment.assignment,
            assignment.apply,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }
}

/// Builds a group handler context from a host's social and lifecycle state plus hub.
pub trait GroupHandlerHostLikeCpp<C> {
    fn group_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> GroupHandlerCxLikeCpp<'a>;
}

fn handle_set_party_leader_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .group_handler_cx_like_cpp(catalogs)
            .handle_set_party_leader(pkt)
            .await;
    })
}

fn handle_set_assistant_leader_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .group_handler_cx_like_cpp(catalogs)
            .handle_set_assistant_leader(pkt)
            .await;
    })
}

fn handle_set_everyone_is_assistant_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .group_handler_cx_like_cpp(catalogs)
            .handle_set_everyone_is_assistant(pkt)
            .await;
    })
}

fn handle_set_party_assignment_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .group_handler_cx_like_cpp(catalogs)
            .handle_set_party_assignment(pkt)
            .await;
    })
}

/// Registers the group leadership/assistant handlers on the packet registry.
pub fn register_group_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_party_leader",
        handler: handle_set_party_leader_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetAssistantLeader,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_assistant_leader",
        handler: handle_set_assistant_leader_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetEveryoneIsAssistant,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_everyone_is_assistant",
        handler: handle_set_everyone_is_assistant_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetPartyAssignment,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_party_assignment",
        handler: handle_set_party_assignment_thunk::<S, C>,
    })?;
    Ok(())
}
