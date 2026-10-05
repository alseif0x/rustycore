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
    ChangeSubGroup, SetAssistantLeader, SetEveryoneIsAssistant, SetPartyAssignment, SetPartyLeader,
    SwapSubGroups,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_social::group::{MAX_RAID_SUBGROUPS_LIKE_CPP, MEMBER_FLAG_ASSISTANT_LIKE_CPP};
use wow_world_core::session::HubMut;
use wow_world_core::session::mailbox::{ApplyGroupSubgroupLikeCppCommand, SessionCommand};
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

    /// C++ `WorldSession::HandleChangeSubGroupOpcode`.
    ///
    /// Returns whether the sender's own subgroup changed, so the host can run
    /// the registry-state publication that still belongs to the World session.
    pub async fn handle_change_sub_group(&mut self, mut pkt: WorldPacket) -> bool {
        let change = match ChangeSubGroup::read(&mut pkt) {
            Ok(change) => change,
            Err(e) => {
                warn!("Bad ChangeSubGroup: {e}");
                return false;
            }
        };

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        if usize::from(change.new_subgroup) >= MAX_RAID_SUBGROUPS_LIKE_CPP {
            return false;
        }

        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return false;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return false;
        };
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            change.party_index,
        ) else {
            return false;
        };

        let outcome = match group_reg.change_member_subgroup_like_cpp(
            group_guid,
            sender_guid,
            change.target_guid,
            change.new_subgroup,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return false,
        };
        let (target_guid, new_subgroup) = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let mut needs_registry_sync = false;
        if target_guid == sender_guid {
            needs_registry_sync = self.apply_group_subgroup_like_cpp(group_guid, new_subgroup);
        } else if let Some(target) = registry.group_presence(target_guid) {
            let _ = registry.deliver_group_state_command_like_cpp(
                target.registration,
                SessionCommand::ApplyGroupSubgroupLikeCpp(ApplyGroupSubgroupLikeCppCommand {
                    group_guid,
                    subgroup: new_subgroup,
                }),
            );
        } else {
            // C++ `Group::ChangeMembersGroup` sets the member's subgroup on the
            // member itself; an unresolvable member reconciles later (#743).
            registry.mark_group_state_reconciliation_like_cpp(target_guid);
        }

        send_party_update(&outcome.group, &registry, vra);
        needs_registry_sync
    }

    /// C++ `WorldSession::HandleSwapSubGroupsOpcode`.
    ///
    /// Returns whether the sender's own subgroup changed, so the host can run
    /// the registry-state publication that still belongs to the World session.
    pub async fn handle_swap_sub_groups(&mut self, mut pkt: WorldPacket) -> bool {
        let swap = match SwapSubGroups::read(&mut pkt) {
            Ok(swap) => swap,
            Err(e) => {
                warn!("Bad SwapSubGroups: {e}");
                return false;
            }
        };

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return false;
        };
        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return false;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return false;
        };
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            swap.party_index,
        ) else {
            return false;
        };

        let outcome = match group_reg.swap_member_subgroups_like_cpp(
            group_guid,
            sender_guid,
            swap.first_target,
            swap.second_target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return false,
        };
        let subgroup_updates = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let mut needs_registry_sync = false;
        for (member_guid, subgroup) in subgroup_updates {
            if member_guid == sender_guid {
                needs_registry_sync |= self.apply_group_subgroup_like_cpp(group_guid, subgroup);
            } else if let Some(member) = registry.group_presence(member_guid) {
                let _ = registry.deliver_group_state_command_like_cpp(
                    member.registration,
                    SessionCommand::ApplyGroupSubgroupLikeCpp(ApplyGroupSubgroupLikeCppCommand {
                        group_guid,
                        subgroup,
                    }),
                );
            } else {
                registry.mark_group_state_reconciliation_like_cpp(member_guid);
            }
        }

        send_party_update(&outcome.group, &registry, vra);
        needs_registry_sync
    }

    /// C++ `Player::SetGroup` + `Player::SetSubGroup` for the sender itself.
    ///
    /// The registry-state publication that follows the canonical update stays a
    /// bounded host seam until it moves with its own owners.
    fn apply_group_subgroup_like_cpp(&mut self, group_guid: u64, subgroup: u8) -> bool {
        self.resolved_group_guid_like_cpp() == Some(group_guid)
            && self
                .social
                .set_owned_player_group_like_cpp(&mut self.hub, Some((group_guid, subgroup)))
    }
}

/// Builds a group handler context from a host's social and lifecycle state plus hub.
pub trait GroupHandlerHostLikeCpp<C> {
    fn group_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> GroupHandlerCxLikeCpp<'a>;

    /// Final publication seam for the subgroup handlers: the owner changed the
    /// sender's own subgroup, and the World session still runs the registry
    /// state sync with its own providers.
    fn sync_player_registry_state_after_group_subgroup_like_cpp(&mut self);
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
fn handle_change_sub_group_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let changed = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_change_sub_group(pkt)
            .await;
        if changed {
            session.sync_player_registry_state_after_group_subgroup_like_cpp();
        }
    })
}

fn handle_swap_sub_groups_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let changed = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_swap_sub_groups(pkt)
            .await;
        if changed {
            session.sync_player_registry_state_after_group_subgroup_like_cpp();
        }
    })
}

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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeSubGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_change_sub_group",
        handler: handle_change_sub_group_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SwapSubGroups,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_swap_sub_groups",
        handler: handle_swap_sub_groups_thunk::<S, C>,
    })?;
    Ok(())
}
