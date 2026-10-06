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

use rand::Rng;
use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_core::guid::HighGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{RandomRoll, RandomRollClient};
use wow_packet::packets::party::{
    ChangeSubGroup, ConvertRaid, GroupDecline, OptOutOfLoot, PartyCommandResult, PartyInviteServer,
    PartyUninvite, SetAssistantLeader, SetEveryoneIsAssistant, SetPartyAssignment, SetPartyLeader,
    SwapSubGroups, party_result,
};
use wow_packet::{ClientPacket, ServerPacket, WorldPacket};
use wow_social::group::{
    AcceptGroupInviteResultLikeCpp, CreateGroupInviteResultLikeCpp, GROUP_CATEGORY_HOME_LIKE_CPP,
    GROUP_TYPE_NONE_LIKE_CPP, GROUP_TYPE_NORMAL_LIKE_CPP, GroupAuthorityErrorLikeCpp, GroupInfo,
    GroupMemberRemovalKindLikeCpp, GroupPersistenceIntentLikeCpp, MAX_RAID_SUBGROUPS_LIKE_CPP,
    MEMBER_FLAG_ASSISTANT_LIKE_CPP,
};
use wow_world_core::session::mailbox::{
    ApplyGroupJoinLikeCppCommand, ApplyGroupRemovalLikeCppCommand,
    ApplyGroupSubgroupLikeCppCommand, SessionCommand,
};
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;
use wow_world_core::session::{
    GroupInvitePolicyLikeCpp, HubMut, HubRef, PacketPublicationAccessLikeCpp,
};
use wow_world_instances::InstanceState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot::LootState;
use wow_world_social::SessionSocialLimits;
use wow_world_social::group_fanout::{
    connected_group_members_like_cpp, current_group_guid_like_cpp,
    queue_visible_gameobjects_or_spellclicks_refresh_like_cpp, send_group_new_leader_like_cpp,
    send_party_update, send_realm_packet_to_player_like_cpp,
    send_realm_party_invite_to_player_like_cpp, target_social_has_inviter_friend_like_cpp,
    target_social_ignores_inviter_like_cpp,
};

/// Resolve C++ `Player::m_group` through the hub's generation-checked canonical
/// Player handle. An unresolved owner never falls back in production.
pub fn resolved_group_guid_like_cpp(
    hub: wow_world_core::session::HubRef<'_>,
    social: &SessionSocialLimits,
) -> Option<u64> {
    let owner = hub.core.player_group_owner_access_like_cpp();
    social.resolved_group_guid_with_access_like_cpp(
        &owner,
        cfg!(any(test, feature = "test-fixtures")),
    )
}

/// Deferred publication tail for one group transition.
///
/// C++ runs the registry-state sync (`Player::SetGroup`/`SetSubGroup`) and the
/// visible-gameobject refresh inside the same operation that mutates the group,
/// between the canonical transition and the final packets. Those two providers
/// still live in the World session, so the owner returns the exact remaining
/// sequence and the host runs it in place, preserving the C++ order.
pub enum GroupPublicationTailLikeCpp {
    /// Nothing left to publish.
    None,
    /// Only the visible-gameobject/spell-click refresh remains.
    VisibilityOnly,
    /// Publish the updated party state without a registry sync.
    PartyUpdateOnly { group: GroupInfo },
    /// Sync the registry state, then publish the updated party state.
    SyncThenPartyUpdate { group: GroupInfo },
    /// Sync, refresh visibility, then send `SMSG_GROUP_UNINVITE`.
    SyncThenVisibilityThenGroupUninvite,
    /// Sync, refresh visibility, then send `SMSG_GROUP_DESTROYED` and the
    /// destroyed party update for `group_guid`.
    SyncThenVisibilityThenGroupDestroyed { group_guid: u64 },
    /// Refresh visibility, persist the accepted-invite intents, then publish the
    /// updated party state.
    VisibilityThenPersistThenPartyUpdate {
        group: GroupInfo,
        group_guid: u64,
        persistence: Vec<GroupPersistenceIntentLikeCpp>,
        refresh_visible_gameobjects_or_spellclicks: bool,
    },
}

/// C++ `WorldSession::GetPlayerMapAndInstance` projection for the invite gate.
pub fn current_player_party_invite_map_instance_like_cpp(
    hub: HubRef<'_>,
    registry: &wow_world_core::player_directory::PlayerRegistry,
    player_guid: ObjectGuid,
) -> (u16, u32) {
    if let Some(key) = hub.core.current_canonical_player_map_key_like_cpp() {
        return (key.map_id.min(u32::from(u16::MAX)) as u16, key.instance_id);
    }

    registry
        .group_presence(player_guid)
        .map(|entry| (entry.map_id, entry.instance_id))
        .unwrap_or_else(|| (hub.core.player_map_id_like_cpp(), 0))
}

/// Borrowed inputs of one group leadership/assistant handler invocation.
pub struct GroupHandlerCxLikeCpp<'a> {
    social: &'a mut SessionSocialLimits,
    lifecycle: &'a SessionLifecycleState,
    loot: &'a mut LootState,
    instances: &'a InstanceState,
    policy: &'a GroupInvitePolicyLikeCpp,
    hub: HubMut<'a>,
}

impl<'a> GroupHandlerCxLikeCpp<'a> {
    pub fn new(
        social: &'a mut SessionSocialLimits,
        lifecycle: &'a SessionLifecycleState,
        loot: &'a mut LootState,
        instances: &'a InstanceState,
        policy: &'a GroupInvitePolicyLikeCpp,
        hub: HubMut<'a>,
    ) -> Self {
        Self {
            social,
            lifecycle,
            loot,
            instances,
            policy,
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

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
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
    pub async fn handle_change_sub_group(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let change = match ChangeSubGroup::read(&mut pkt) {
            Ok(change) => change,
            Err(e) => {
                warn!("Bad ChangeSubGroup: {e}");
                return GroupPublicationTailLikeCpp::None;
            }
        };

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        if usize::from(change.new_subgroup) >= MAX_RAID_SUBGROUPS_LIKE_CPP {
            return GroupPublicationTailLikeCpp::None;
        }

        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            change.party_index,
        ) else {
            return GroupPublicationTailLikeCpp::None;
        };

        let outcome = match group_reg.change_member_subgroup_like_cpp(
            group_guid,
            sender_guid,
            change.target_guid,
            change.new_subgroup,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return GroupPublicationTailLikeCpp::None,
        };
        let (target_guid, new_subgroup) = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        if target_guid == sender_guid {
            return if self.apply_group_subgroup_like_cpp(group_guid, new_subgroup) {
                GroupPublicationTailLikeCpp::SyncThenPartyUpdate {
                    group: outcome.group,
                }
            } else {
                GroupPublicationTailLikeCpp::PartyUpdateOnly {
                    group: outcome.group,
                }
            };
        }
        if let Some(target) = registry.group_presence(target_guid) {
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

        GroupPublicationTailLikeCpp::PartyUpdateOnly {
            group: outcome.group,
        }
    }

    /// C++ `WorldSession::HandleSwapSubGroupsOpcode`.
    ///
    /// Returns whether the sender's own subgroup changed, so the host can run
    /// the registry-state publication that still belongs to the World session.
    pub async fn handle_swap_sub_groups(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let swap = match SwapSubGroups::read(&mut pkt) {
            Ok(swap) => swap,
            Err(e) => {
                warn!("Bad SwapSubGroups: {e}");
                return GroupPublicationTailLikeCpp::None;
            }
        };

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let vra = self.hub.shared().core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            swap.party_index,
        ) else {
            return GroupPublicationTailLikeCpp::None;
        };

        let outcome = match group_reg.swap_member_subgroups_like_cpp(
            group_guid,
            sender_guid,
            swap.first_target,
            swap.second_target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return GroupPublicationTailLikeCpp::None,
        };
        let subgroup_updates = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let mut own_subgroup_changed = false;
        for (member_guid, subgroup) in subgroup_updates {
            if member_guid == sender_guid {
                own_subgroup_changed |= self.apply_group_subgroup_like_cpp(group_guid, subgroup);
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

        if own_subgroup_changed {
            GroupPublicationTailLikeCpp::SyncThenPartyUpdate {
                group: outcome.group,
            }
        } else {
            GroupPublicationTailLikeCpp::PartyUpdateOnly {
                group: outcome.group,
            }
        }
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

    /// C++ `WorldSession::HandleLeaveGroupOpcode`.
    ///
    /// Returns `(registry_sync, visibility_refresh)`: the two publications that
    /// still belong to the World session run in the host after the owner's
    /// canonical removal.
    pub async fn handle_leave_group(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };

        let Some(my_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let pending_invites = self.hub.shared().core.pending_invites().cloned();
        let vra = self.hub.shared().core.virtual_realm_address();

        // 1. Find the real group or the C++ `GroupInvite` we're currently in.
        let real_group_guid = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            my_guid,
            party_index,
        );
        let pending_invite = pending_invites
            .as_ref()
            .and_then(|pending| pending.get(&my_guid));

        if real_group_guid.is_none() && pending_invite.is_none() {
            return GroupPublicationTailLikeCpp::None;
        };

        if self
            .hub
            .shared()
            .player_in_represented_battleground_like_cpp()
        {
            self.publication_like_cpp()
                .send_packet_realm(&PartyCommandResult {
                    name: String::new(),
                    command: 0,
                    result: party_result::INVITE_RESTRICTED,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
            return GroupPublicationTailLikeCpp::None;
        }

        let player_name = self.hub.shared().player_name_like_cpp().unwrap_or_default();

        if real_group_guid.is_none() {
            if let (Some(pending_invites), Some(invite)) =
                (pending_invites.as_ref(), pending_invite)
            {
                if invite.leader_guid == my_guid {
                    self.publication_like_cpp()
                        .send_packet_realm(&PartyCommandResult {
                            name: player_name,
                            command: 2,
                            result: party_result::OK,
                            result_data: 0,
                            result_guid: ObjectGuid::EMPTY,
                        });
                    group_reg.cancel_pending_group_like_cpp(pending_invites, invite);
                }
            }
            return GroupPublicationTailLikeCpp::None;
        }
        let gid = real_group_guid.expect("checked above");

        self.publication_like_cpp()
            .send_packet_realm(&PartyCommandResult {
                name: player_name,
                command: 2,
                result: party_result::OK,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });

        // 2. Remove self from the group through the canonical owner. Connected
        // successor candidates are facts; membership and leader choice are
        // revalidated under the group shard.
        let connected_members = group_reg
            .get(&gid)
            .map(|group| connected_group_members_like_cpp(&group, &registry))
            .unwrap_or_default();
        let outcome = match group_reg.remove_member_like_cpp(
            gid,
            my_guid,
            GroupMemberRemovalKindLikeCpp::Leave,
            &connected_members,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return GroupPublicationTailLikeCpp::None,
        };
        let dissolve_remaining = outcome
            .facts
            .disbanded
            .then(|| outcome.facts.remaining_members.clone());
        self.lifecycle
            .persist_group_intents_like_cpp(gid, outcome.persistence)
            .await;

        if let Some(remaining) = dissolve_remaining {
            // Group dissolved — notify last remaining member (if any).
            if let Some(&last_guid) = remaining.first() {
                if let Some(last) = registry.group_presence(last_guid) {
                    let command = ApplyGroupRemovalLikeCppCommand {
                        group_guid: gid,
                        category: GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: GROUP_TYPE_NONE_LIKE_CPP,
                        send_group_destroyed: true,
                        send_group_uninvite: false,
                        refresh_visible_gameobjects_or_spellclicks: true,
                    };
                    let _ = registry.deliver_group_state_command_like_cpp(
                        last.registration,
                        SessionCommand::ApplyGroupRemovalLikeCpp(command),
                    );
                } else {
                    // C++ `Group::Disband` clears every connected member's
                    // group in the same operation (#743).
                    registry.mark_group_state_reconciliation_like_cpp(last_guid);
                }
            }
            self.detach_self_from_group_like_cpp();
            return GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupUninvite;
        }

        // 3. Send updated PartyUpdate to remaining members.
        send_party_update(&outcome.group, &registry, vra);

        // 4. Uninvite self.
        self.detach_self_from_group_like_cpp();
        GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupUninvite
    }

    /// C++ `WorldSession::HandleConvertRaidOpcode`.
    ///
    /// Returns `(registry_sync, visibility_refresh)`.
    pub async fn handle_convert_raid(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let convert = match ConvertRaid::read(&mut pkt) {
            Ok(convert) => convert,
            Err(e) => {
                warn!("Bad ConvertRaid: {e}");
                return GroupPublicationTailLikeCpp::None;
            }
        };

        let Some(my_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            my_guid,
            None,
        ) else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let vra = self.hub.shared().core.virtual_realm_address();

        let outcome = match group_reg.convert_group_like_cpp(group_guid, my_guid, convert.raid) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::GroupTooLarge) => {
                self.publication_like_cpp()
                    .send_packet_realm(&PartyCommandResult {
                        name: String::new(),
                        command: 0,
                        result: party_result::OK,
                        result_data: 0,
                        result_guid: ObjectGuid::EMPTY,
                    });
                return GroupPublicationTailLikeCpp::None;
            }
            Err(_) => return GroupPublicationTailLikeCpp::None,
        };
        self.publication_like_cpp()
            .send_packet_realm(&PartyCommandResult {
                name: String::new(),
                command: 0,
                result: party_result::OK,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        // `queue_visible...` may wait on a full member command channel. Clone
        // the value and release DashMap's read guard before the first await so
        // unrelated group mutations are never stalled behind that backpressure.
        send_party_update(&outcome.group, &registry, vra);
        queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(
            &outcome.group,
            &registry,
            my_guid,
        )
        .await;
        GroupPublicationTailLikeCpp::VisibilityOnly
    }

    /// C++ `WorldSession::SendPartyResult(..., PARTY_OP_INVITE, ...)`.
    fn send_party_result_like_cpp(&self, name: String, result: u8) {
        self.publication_like_cpp()
            .send_packet_realm(&PartyCommandResult {
                name,
                command: 0, // Invite
                result,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });
    }

    /// C++ `WorldSession::HandlePartyInviteOpcode`.
    pub async fn handle_party_invite(&mut self, mut pkt: WorldPacket) {
        tracing::info!(
            account = self.hub.shared().core.account_id,
            "handle_party_invite called"
        );
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let _ = pkt.reset_bits(); // ResetBitPos / flush partial byte

        let name_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("PartyInvite: name_len read error: {}", e);
                return;
            }
        };
        let realm_len = match pkt.read_bits(9) {
            Ok(n) => n as usize,
            Err(e) => {
                warn!("PartyInvite: realm_len read error: {}", e);
                return;
            }
        };

        let proposed_roles = pkt.read_uint32().unwrap_or(0);

        let _target_guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(e) => {
                warn!("PartyInvite: target_guid read error: {}", e);
                return;
            }
        };
        let target_name = match pkt.read_string(name_len) {
            Ok(s) => s,
            Err(e) => {
                warn!("PartyInvite: target_name read error: {}", e);
                return;
            }
        };
        let _realm_name = pkt.read_string(realm_len).unwrap_or_default();
        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };
        tracing::info!(account = self.hub.shared().core.account_id, target_name = %target_name, "PartyInvite parsed");

        // — setup —
        let my_guid = match self.hub.shared().core.player_guid() {
            Some(g) => g,
            None => return,
        };

        macro_rules! send_result {
            ($result:expr) => {
                self.publication_like_cpp()
                    .send_packet_realm(&PartyCommandResult {
                        name: target_name.clone(),
                        command: 0, // Invite
                        result: $result,
                        result_data: 0,
                        result_guid: ObjectGuid::EMPTY,
                    });
            };
        }

        // 2. Target must exist in the player registry (lookup by name — robust against GUID mismatch).
        let registry = match self.hub.shared().core.player_registry() {
            Some(r) => r,
            None => return,
        };

        // Find target by name (case-insensitive), same pattern as whisper handler.
        let target_snapshot = match registry.social_recipient_by_name(&target_name) {
            Some(target) => target,
            None => {
                warn!(
                    "PartyInvite: target '{}' not found in registry",
                    target_name
                );
                self.send_party_result_like_cpp(target_name.clone(), party_result::BAD_PLAYER_NAME);
                return;
            }
        };
        let real_target_guid = target_snapshot.guid;

        // Don't invite yourself (compare by real GUID from registry).
        if real_target_guid == my_guid {
            self.send_party_result_like_cpp(target_name.clone(), party_result::BAD_PLAYER_NAME);
            return;
        }

        // C++ `HandlePartyInviteOpcode` rejects inviting GM targets unless
        // `GM.AllowInvite` / `CONFIG_ALLOW_GM_GROUP` is enabled.
        if !self.policy.allow_gm_group
            && self.hub.shared().player_is_game_master_like_cpp() != Some(true)
            && target_snapshot.is_game_master
        {
            self.send_party_result_like_cpp(target_name.clone(), party_result::BAD_PLAYER_NAME);
            return;
        }

        if !self.policy.allow_two_side_interaction
            && self.hub.shared().player_is_game_master_like_cpp() != Some(true)
            && player_team_for_race_cpp(self.hub.shared().player_race_like_cpp())
                != player_team_for_race_cpp(target_snapshot.race)
        {
            self.send_party_result_like_cpp(target_name.clone(), party_result::WRONG_FACTION);
            return;
        }

        let (inviter_map_id, inviter_instance_id) =
            current_player_party_invite_map_instance_like_cpp(self.hub.shared(), registry, my_guid);
        if inviter_instance_id != 0
            && target_snapshot.instance_id != 0
            && inviter_instance_id != target_snapshot.instance_id
            && inviter_map_id == target_snapshot.map_id
        {
            self.send_party_result_like_cpp(
                target_name.clone(),
                party_result::TARGET_NOT_IN_INSTANCE,
            );
            return;
        }

        if target_snapshot.instance_id != 0 {
            let Some(inviter_difficulty_id) = self
                .instances
                .resolved_dungeon_difficulty_id_like_cpp(self.hub.shared())
            else {
                self.send_party_result_like_cpp(target_name.clone(), party_result::IGNORING_YOU);
                return;
            };
            if target_snapshot.dungeon_difficulty_id != inviter_difficulty_id {
                self.send_party_result_like_cpp(target_name.clone(), party_result::IGNORING_YOU);
                return;
            }
        }

        let social_port = self.lifecycle.social_persistence_port_like_cpp();
        if target_social_ignores_inviter_like_cpp(
            social_port.clone(),
            real_target_guid,
            my_guid,
            self.hub.shared().core.account_id,
        )
        .await
        {
            self.send_party_result_like_cpp(target_name.clone(), party_result::IGNORING_YOU);
            return;
        }

        if u32::from(self.hub.shared().player_level_like_cpp()) < self.policy.minimum_level
            && !target_social_has_inviter_friend_like_cpp(social_port, real_target_guid, my_guid)
                .await
        {
            self.send_party_result_like_cpp(target_name.clone(), party_result::INVITE_RESTRICTED);
            return;
        }

        // 3. The owner revalidates pending/group/category/capacity state and
        // records the invite as one transition.
        let pending = match self.hub.shared().core.pending_invites() {
            Some(p) => p,
            None => return,
        };

        let inviter_name = self.hub.shared().player_name_like_cpp().unwrap_or_default();
        let vra = self.hub.shared().core.virtual_realm_address();
        let (realm_name, realm_name_normalized) = self
            .hub
            .shared()
            .core
            .realm_names_for_address_like_cpp(vra)
            .map(|(actual, normalized)| (actual.to_string(), normalized.to_string()))
            .unwrap_or_default();

        let group_reg = match self.hub.shared().core.group_registry() {
            Some(r) => r,
            None => return,
        };

        let inviter_group_guid = current_group_guid_like_cpp(
            group_reg,
            self.resolved_group_guid_like_cpp(),
            my_guid,
            party_index,
        );
        let lookup_category = party_index.unwrap_or(GROUP_CATEGORY_HOME_LIKE_CPP);
        let invite = match group_reg.create_invite_like_cpp(
            pending,
            my_guid,
            real_target_guid,
            inviter_group_guid,
            lookup_category,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ) {
            CreateGroupInviteResultLikeCpp::Created(invite) => invite,
            CreateGroupInviteResultLikeCpp::TargetAlreadyInvited => {
                self.send_party_result_like_cpp(
                    target_name.clone(),
                    party_result::ALREADY_IN_GROUP,
                );
                return;
            }
            CreateGroupInviteResultLikeCpp::TargetAlreadyGrouped => {
                self.send_party_result_like_cpp(
                    target_name.clone(),
                    party_result::ALREADY_IN_GROUP,
                );
                let invite = PartyInviteServer {
                    can_accept: false,
                    proposed_roles: proposed_roles as u8,
                    inviter_name: inviter_name.clone(),
                    inviter_guid: my_guid,
                    inviter_bnet_account_guid: ObjectGuid::create_global(
                        HighGuid::WowAccount,
                        0,
                        self.hub.shared().core.account_id as i64,
                    ),
                    virtual_realm_address: vra,
                    realm_name: realm_name.clone(),
                    realm_name_normalized: realm_name_normalized.clone(),
                };
                let _ = send_realm_packet_to_player_like_cpp(
                    registry,
                    target_snapshot.registration,
                    real_target_guid,
                    invite.to_bytes(),
                )
                .await;
                return;
            }
            CreateGroupInviteResultLikeCpp::InviterNotLeaderOrAssistant => {
                self.send_party_result_like_cpp(target_name.clone(), party_result::NOT_LEADER);
                return;
            }
            CreateGroupInviteResultLikeCpp::GroupFull => {
                self.send_party_result_like_cpp(target_name.clone(), party_result::GROUP_FULL);
                return;
            }
            CreateGroupInviteResultLikeCpp::MissingInviterGroup
            | CreateGroupInviteResultLikeCpp::WrongCategory => return,
        };

        // 7. Send invite dialog to the target.
        let invite_packet = PartyInviteServer {
            can_accept: true,
            proposed_roles: proposed_roles as u8,
            inviter_name: inviter_name.clone(),
            inviter_guid: my_guid,
            inviter_bnet_account_guid: ObjectGuid::create_global(
                HighGuid::WowAccount,
                0,
                self.hub.shared().core.account_id as i64,
            ),
            virtual_realm_address: vra,
            realm_name,
            realm_name_normalized,
        };
        if !send_realm_party_invite_to_player_like_cpp(
            registry,
            target_snapshot.registration,
            real_target_guid,
            invite_packet.to_bytes(),
        )
        .await
        {
            group_reg.cancel_invite_like_cpp(pending, real_target_guid, invite);
            self.send_party_result_like_cpp(target_name.clone(), party_result::BAD_PLAYER_NAME);
            return;
        }

        // 7. Confirm back to self.
        self.publication_like_cpp()
            .send_packet_realm(&PartyCommandResult {
                name: target_name,
                command: 0,
                result: party_result::OK,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });
    }

    /// C++ `WorldSession::HandleRandomRollOpcode`.
    pub async fn handle_random_roll(&mut self, mut pkt: WorldPacket) {
        let roll = match RandomRollClient::read(&mut pkt) {
            Ok(roll) => roll,
            Err(e) => {
                warn!("Bad RandomRoll: {e}");
                return;
            }
        };

        if roll.min > roll.max || roll.max > 1_000_000 {
            return;
        }

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return;
        };

        let result = rand::thread_rng().gen_range(roll.min..=roll.max);
        let response = RandomRoll {
            roller: sender_guid,
            roller_wow_account: ObjectGuid::new(
                (HighGuid::WowAccount as i64) << 58,
                i64::from(self.hub.shared().core.account_id),
            ),
            min: roll.min,
            max: roll.max,
            result,
        };
        let bytes = response.to_bytes();

        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            self.publication_like_cpp().send_packet(&response);
            return;
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            None,
        ) else {
            self.publication_like_cpp().send_packet(&response);
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            self.publication_like_cpp().send_packet(&response);
            return;
        };

        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            self.publication_like_cpp().send_packet(&response);
            return;
        };

        let mut sent_to_sender = false;
        // C++ `group->BroadcastPacket(randomRoll.Write(), false)` includes the roller.
        for member_guid in &group.members {
            if let Some(member) = registry.group_presence(*member_guid) {
                let _ = registry.send_current_packet(member.registration, bytes.clone());
                if *member_guid == sender_guid {
                    sent_to_sender = true;
                }
            }
        }

        if !sent_to_sender {
            self.publication_like_cpp().send_packet(&response);
        }
    }

    /// C++ `WorldSession::HandleOptOutOfLootOpcode`.
    pub async fn handle_opt_out_of_loot(&mut self, mut pkt: WorldPacket) {
        let opt_out = match OptOutOfLoot::read(&mut pkt) {
            Ok(opt_out) => opt_out,
            Err(e) => {
                warn!("Bad OptOutOfLoot: {e}");
                return;
            }
        };

        if self.hub.shared().core.player_guid().is_none() {
            if opt_out.pass_on_loot {
                warn!("CMSG_OPT_OUT_OF_LOOT value<>0 for not-loaded character");
            }
            return;
        }

        let _ = self
            .loot
            .set_pass_on_group_loot_like_cpp(&mut self.hub, opt_out.pass_on_loot);
    }

    /// C++ `WorldSession::HandlePartyInviteResponseOpcode`.
    pub async fn handle_party_invite_response(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let accept = pkt.read_bit().unwrap_or(false);
        let has_roles = pkt.read_bit().unwrap_or(false);

        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };
        if has_roles {
            let _ = pkt.read_uint8();
        }

        let Some(my_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let my_name = self.hub.shared().player_name_like_cpp().unwrap_or_default();

        // Clone Arcs immediately so we hold no borrow on `self` later.
        let Some(pending) = self.hub.shared().core.pending_invites().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };

        // 1. Must have a pending C++ `GroupInvite`.
        let Some(invite) = pending.get(&my_guid) else {
            return GroupPublicationTailLikeCpp::None;
        };

        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        // 2. Declined?
        if !accept {
            let Some(invite) = group_reg.decline_invite_like_cpp(&pending, my_guid, party_index)
            else {
                return GroupPublicationTailLikeCpp::None;
            };
            if let Some(leader) = registry.group_presence(invite.leader_guid) {
                let decline = GroupDecline { name: my_name };
                let _ = send_realm_packet_to_player_like_cpp(
                    &registry,
                    leader.registration,
                    invite.leader_guid,
                    decline.to_bytes(),
                )
                .await;
            }
            return GroupPublicationTailLikeCpp::None;
        }

        let leader = registry.group_presence(invite.leader_guid);
        let (group, persistence, refresh_visible_gameobjects_or_spellclicks) = match group_reg
            .accept_invite_like_cpp(
                &pending,
                my_guid,
                party_index,
                leader.as_ref().map(|_| invite.leader_guid),
            ) {
            AcceptGroupInviteResultLikeCpp::NoInvite
            | AcceptGroupInviteResultLikeCpp::WrongCategory
            | AcceptGroupInviteResultLikeCpp::AddFailed
            | AcceptGroupInviteResultLikeCpp::AlreadyMember
            | AcceptGroupInviteResultLikeCpp::MissingGroup
            | AcceptGroupInviteResultLikeCpp::MissingLeader => {
                return GroupPublicationTailLikeCpp::None;
            }
            AcceptGroupInviteResultLikeCpp::SelfInvite => {
                warn!(
                    player = %my_guid,
                    "HandlePartyInviteResponse: player tried to accept an invite to his own group"
                );
                return GroupPublicationTailLikeCpp::None;
            }
            AcceptGroupInviteResultLikeCpp::GroupFull => {
                self.publication_like_cpp()
                    .send_packet_realm(&PartyCommandResult {
                        name: String::new(),
                        command: 0,
                        result: party_result::GROUP_FULL,
                        result_data: 0,
                        result_guid: ObjectGuid::EMPTY,
                    });
                return GroupPublicationTailLikeCpp::None;
            }
            AcceptGroupInviteResultLikeCpp::JoinedExisting {
                group,
                subgroup: _,
                persistence,
            } => {
                let is_raid_group = group.is_raid_group();
                (group, persistence, is_raid_group)
            }
            AcceptGroupInviteResultLikeCpp::Created {
                group,
                subgroup: _,
                persistence,
            } => {
                if let Some(leader) = leader.as_ref() {
                    let _ = registry.deliver_group_state_command_like_cpp(
                        leader.registration,
                        SessionCommand::ApplyGroupJoinLikeCpp(ApplyGroupJoinLikeCppCommand {
                            group_guid: group.group_guid,
                            category: group.group_category_like_cpp(),
                            party_type: GROUP_TYPE_NORMAL_LIKE_CPP,
                            subgroup: 0,
                            refresh_visible_gameobjects_or_spellclicks: false,
                        }),
                    );
                } else {
                    // The leader is connected but not resolvable this instant
                    // (transfer, detached residence). C++ installs the group on
                    // the leader inside the same operation, so the obligation is
                    // recorded rather than dropped (#743).
                    registry.mark_group_state_reconciliation_like_cpp(invite.leader_guid);
                }
                (group, persistence, false)
            }
        };
        let group_guid = group.group_guid;

        // Attach C++ `Player::m_group` after the Group owner accepted us.
        if let Some(subgroup) = group_reg.get(&group_guid).and_then(|group| {
            group
                .member_slot_like_cpp(my_guid)
                .map(|slot| slot.subgroup)
        }) {
            self.social
                .apply_group_join_like_cpp(&mut self.hub, group_guid, subgroup);
        }
        if let Some(group) = group_reg.get(&group_guid) {
            self.social.send_player_party_type_update_like_cpp(
                self.hub.shared(),
                group.group_category_like_cpp(),
                GROUP_TYPE_NORMAL_LIKE_CPP,
            );
        }
        self.social
            .sync_player_registry_party_member_party_type_like_cpp(self.hub.shared());

        GroupPublicationTailLikeCpp::VisibilityThenPersistThenPartyUpdate {
            group,
            group_guid,
            persistence,
            refresh_visible_gameobjects_or_spellclicks,
        }
    }

    /// C++ `WorldSession::SendPartyResult(..., PARTY_OP_UNINVITE, ...)`.
    fn send_party_uninvite_result_like_cpp(&self, result: u8) {
        self.publication_like_cpp()
            .send_packet_realm(&PartyCommandResult {
                name: String::new(),
                command: 1, // C++ PARTY_OP_UNINVITE
                result,
                result_data: 0,
                // C++ `WorldSession::SendPartyResult` always leaves `ResultGUID`
                // empty (`GroupHandler.cpp:53`).
                result_guid: ObjectGuid::EMPTY,
            });
    }

    /// C++ `WorldSession::HandlePartyUninviteOpcode`.
    pub async fn handle_party_uninvite(
        &mut self,
        mut pkt: WorldPacket,
    ) -> GroupPublicationTailLikeCpp {
        let uninvite = match PartyUninvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!("Bad PartyUninvite: {error}");
                return GroupPublicationTailLikeCpp::None;
            }
        };

        let Some(sender_guid) = self.hub.shared().core.player_guid() else {
            return GroupPublicationTailLikeCpp::None;
        };
        if uninvite.target_guid == sender_guid {
            return GroupPublicationTailLikeCpp::None;
        }

        let Some(group_reg) = self.hub.shared().core.group_registry().cloned() else {
            self.send_party_uninvite_result_like_cpp(party_result::NOT_IN_GROUP);
            return GroupPublicationTailLikeCpp::None;
        };
        let Some(registry) = self.hub.shared().core.player_registry().cloned() else {
            return GroupPublicationTailLikeCpp::None;
        };
        let pending_invites = self.hub.shared().core.pending_invites().cloned();
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            uninvite.party_index,
        ) else {
            self.send_party_uninvite_result_like_cpp(party_result::NOT_IN_GROUP);
            return GroupPublicationTailLikeCpp::None;
        };

        let Some(group_snapshot) = group_reg.get(&group_guid) else {
            return GroupPublicationTailLikeCpp::None;
        };
        let target_has_loot_rolls = registry
            .group_presence(uninvite.target_guid)
            .is_some_and(|target| target.has_active_loot_rolls);
        let sender_map_id = self.hub.shared().core.player_map_id_like_cpp();
        let sender_instance_id = self
            .hub
            .shared()
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let any_member_in_combat = group_snapshot.members.iter().any(|member_guid| {
            if *member_guid == sender_guid {
                self.hub.shared().resolved_in_combat_like_cpp() != Some(false)
            } else {
                registry.group_presence(*member_guid).is_some_and(|member| {
                    member.in_combat
                        && member.map_id == sender_map_id
                        && member.instance_id == sender_instance_id
                })
            }
        });
        let outcome = match group_reg.remove_member_like_cpp(
            group_guid,
            uninvite.target_guid,
            GroupMemberRemovalKindLikeCpp::Kick {
                actor_guid: sender_guid,
                actor_in_battleground: self
                    .hub
                    .shared()
                    .player_in_represented_battleground_like_cpp(),
                target_has_loot_rolls,
                any_member_in_actor_map_combat: any_member_in_combat,
            },
            &[],
        ) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::LfgBootLimit) => {
                self.send_party_uninvite_result_like_cpp(party_result::PARTY_LFG_BOOT_LIMIT);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootTooFewPlayers) => {
                self.send_party_uninvite_result_like_cpp(
                    party_result::PARTY_LFG_BOOT_TOO_FEW_PLAYERS,
                );
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootDungeonComplete) => {
                self.send_party_uninvite_result_like_cpp(
                    party_result::PARTY_LFG_BOOT_DUNGEON_COMPLETE,
                );
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootLootRolls) => {
                self.send_party_uninvite_result_like_cpp(party_result::PARTY_LFG_BOOT_LOOT_ROLLS);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootInCombat) => {
                self.send_party_uninvite_result_like_cpp(party_result::PARTY_LFG_BOOT_IN_COMBAT);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant)
            | Err(GroupAuthorityErrorLikeCpp::TargetIsLeader) => {
                self.send_party_uninvite_result_like_cpp(party_result::NOT_LEADER);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::InviteRestricted) => {
                self.send_party_uninvite_result_like_cpp(party_result::INVITE_RESTRICTED);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::MissingMember) => {
                if let Some(pending_invites) = pending_invites.as_ref() {
                    if let Some(invite) = pending_invites
                        .get(&uninvite.target_guid)
                        .filter(|invite| invite.group_guid == Some(group_guid))
                    {
                        group_reg.cancel_invite_like_cpp(
                            pending_invites,
                            uninvite.target_guid,
                            invite,
                        );
                        return GroupPublicationTailLikeCpp::None;
                    }
                }
                self.send_party_uninvite_result_like_cpp(party_result::TARGET_NOT_IN_GROUP);
                return GroupPublicationTailLikeCpp::None;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgKickOwnedByVote) => {
                return GroupPublicationTailLikeCpp::None;
            }
            Err(_) => return GroupPublicationTailLikeCpp::None,
        };
        let should_disband = outcome.facts.disbanded;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let cleanup_command = ApplyGroupRemovalLikeCppCommand {
            group_guid,
            category: GROUP_CATEGORY_HOME_LIKE_CPP,
            party_type: GROUP_TYPE_NONE_LIKE_CPP,
            send_group_destroyed: should_disband,
            send_group_uninvite: !should_disband,
            refresh_visible_gameobjects_or_spellclicks: true,
        };
        if let Some(target) = registry.group_presence(uninvite.target_guid) {
            let _ = registry.deliver_group_state_command_like_cpp(
                target.registration,
                SessionCommand::ApplyGroupRemovalLikeCpp(cleanup_command),
            );
        } else {
            // C++ `Group::RemoveMember` clears `Player::m_group` on the target
            // itself; an unresolvable target keeps the obligation (#743).
            registry.mark_group_state_reconciliation_like_cpp(uninvite.target_guid);
        }

        if should_disband {
            self.detach_self_from_group_like_cpp();
            return GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupDestroyed {
                group_guid,
            };
        }

        send_party_update(
            &outcome.group,
            &registry,
            self.hub.shared().core.virtual_realm_address(),
        );
        GroupPublicationTailLikeCpp::None
    }

    /// C++ `Player::SetGroup(nullptr)` + `ClearSubGroup` for the sender.
    fn detach_self_from_group_like_cpp(&mut self) {
        self.social
            .set_owned_player_group_like_cpp(&mut self.hub, None);
        self.social
            .clear_represented_group_subgroup_like_cpp(&mut self.hub);
        self.social.send_player_party_type_update_like_cpp(
            self.hub.shared(),
            GROUP_CATEGORY_HOME_LIKE_CPP,
            GROUP_TYPE_NONE_LIKE_CPP,
        );
    }
}

/// Builds a group handler context from a host's social and lifecycle state plus hub.
pub trait GroupHandlerHostLikeCpp<C> {
    fn group_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> GroupHandlerCxLikeCpp<'a>;

    /// Runs the deferred publication tail in C++ order. The registry-state
    /// sync and the visibility refresh still need the World session's stats,
    /// loot, control and map providers, so the host executes them in place.
    fn run_group_publication_tail_like_cpp<'a>(
        &'a mut self,
        tail: GroupPublicationTailLikeCpp,
    ) -> HandlerFuture<'a, ()>;
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
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_change_sub_group(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
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
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_swap_sub_groups(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
    })
}

fn handle_leave_group_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_leave_group(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
    })
}

fn handle_convert_raid_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_convert_raid(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
    })
}

fn handle_party_uninvite_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_party_uninvite(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
    })
}

fn handle_party_invite_response_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: GroupHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let tail = session
            .group_handler_cx_like_cpp(catalogs)
            .handle_party_invite_response(pkt)
            .await;
        session.run_group_publication_tail_like_cpp(tail).await;
    })
}

fn handle_random_roll_thunk<'a, S, C>(
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
            .handle_random_roll(pkt)
            .await;
    })
}

fn handle_opt_out_of_loot_thunk<'a, S, C>(
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
            .handle_opt_out_of_loot(pkt)
            .await;
    })
}

fn handle_party_invite_thunk<'a, S, C>(
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
            .handle_party_invite(pkt)
            .await;
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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LeaveGroup,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_leave_group",
        handler: handle_leave_group_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ConvertRaid,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_convert_raid",
        handler: handle_convert_raid_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::PartyUninvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_uninvite",
        handler: handle_party_uninvite_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInviteResponse,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite_response",
        handler: handle_party_invite_response_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::RandomRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_random_roll",
        handler: handle_random_roll_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::OptOutOfLoot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_opt_out_of_loot",
        handler: handle_opt_out_of_loot_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::PartyInvite,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_party_invite",
        handler: handle_party_invite_thunk::<S, C>,
    })?;
    Ok(())
}
