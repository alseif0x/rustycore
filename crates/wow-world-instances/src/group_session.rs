// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Applying a committed group state change on the member it belongs to, the
//! #743 group-state reconciliation and the deferred group publication tail.
//!
//! These are the receiving half of the transitions the group authority
//! performs in `wow-social`. C++ runs them inside `Group::RemoveMember`
//! (`Group.cpp:550`), `Group::Disband` (`Group.cpp:713`),
//! `Group::ChangeMembersGroup` and `Group::SetDungeonDifficultyID`, reaching
//! every connected member's `Player` directly. RustyCore performs them on the
//! member's own session so its admission phase, canonical Player guard and
//! publication order are preserved.
//!
//! Each handler therefore owns one obligation: apply the change, or record
//! that the member must converge on the authority. Nothing here may drop a
//! state change silently (#743).
//!
//! Moved verbatim from the World `handlers/group/commands.rs`,
//! `session/social/group.rs`, `session/instances/difficulty.rs` and
//! `session/group_application_handler_contexts.rs` (#1363 slice 4d). The
//! World session's social-state wrappers stay for their other World callers;
//! the moved bodies call the same `SessionSocialLimits` operation over the
//! same split.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_social::group::{GroupDifficultyKindLikeCpp, GroupRegistry};
use wow_world_core::session::mailbox::{
    ApplyGroupDifficultyLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyGroupSubgroupLikeCppCommand,
    SendPartyUpdateLikeCppCommand,
};
use wow_world_core::session::{HubMut, HubRef, SessionState};
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;

use crate::{GroupPublicationTailLikeCpp, InstanceSessionHostLikeCpp};

/// The World operations the moved group bodies still call; each is the
/// same-named `WorldSession` operation, or the named field or split.
pub trait GroupSessionHostLikeCpp: InstanceSessionHostLikeCpp {
    /// `crate::session::split_social_ref`.
    fn split_social_ref(&self) -> (&SessionSocialLimits, HubRef<'_>);
    /// `crate::session::split_social_mut`.
    fn split_social_mut(&mut self) -> (&mut SessionSocialLimits, HubMut<'_>);
    /// The session's `lifecycle` field.
    fn lifecycle(&self) -> &SessionLifecycleState;
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize;
    fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    );
}

pub fn handle_apply_group_removal_command_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    command: ApplyGroupRemovalLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        // The command reached a phase that cannot apply it. C++ clears
        // `Player::m_group` inside `Group::RemoveMember`, so the change is
        // deferred to reconciliation instead of dropped (#743).
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }
    if host.resolved_group_guid_like_cpp() != Some(command.group_guid) {
        // Already in another group, or already removed. Nothing to undo,
        // but the mark keeps an unrelated pending divergence honest.
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }
    // A removal that raced a re-join into the same group must not clear a
    // membership the authority still holds.
    if host.authoritative_group_membership_like_cpp() == Some(command.group_guid) {
        return;
    }

    let _ = {
        let (state, mut hub) = host.split_social_mut();
        state.set_owned_player_group_like_cpp(&mut hub, None)
    };
    {
        let (state, hub) = host.split_social_ref();
        state.send_player_party_type_update_like_cpp(hub, command.category, command.party_type)
    };
    host.sync_player_registry_state_like_cpp();

    if command.refresh_visible_gameobjects_or_spellclicks {
        let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }
    if command.send_group_destroyed {
        host.core()
            .send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
    }
    if command.send_group_uninvite {
        host.core()
            .send_packet_realm(&wow_packet::packets::party::GroupUninvite);
    }
    // C++ `Group::RemoveMember` (`Group.cpp:654-655`) and `Group::Disband`
    // (`Group.cpp:746`) both finish by sending the removed player the
    // destroyed `PartyUpdate` so its client tears down the party frames.
    if command.send_group_destroyed || command.send_group_uninvite {
        send_destroyed_group_party_update_like_cpp(host, command.group_guid, command.category);
    }
}

pub fn handle_apply_group_join_command_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    command: ApplyGroupJoinLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }

    {
        let (s, mut h) = host.split_social_mut();
        s.apply_group_join_like_cpp(&mut h, command.group_guid, command.subgroup)
    };
    {
        let (state, hub) = host.split_social_ref();
        state.send_player_party_type_update_like_cpp(hub, command.category, command.party_type)
    };

    if command.refresh_visible_gameobjects_or_spellclicks {
        let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }
}

pub fn handle_send_party_update_command_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    mut command: SendPartyUpdateLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        return;
    }
    if host.player_guid() != Some(command.recipient) {
        return;
    }

    let Some(sequence_num) = ({
        let (state, mut hub) = host.split_social_mut();
        state.next_group_update_sequence_number_like_cpp(&mut hub, command.party_update.party_index)
    }) else {
        return;
    };
    command.party_update.sequence_num = sequence_num;
    // `SMSG_PARTY_UPDATE` and `SMSG_PARTY_MEMBER_FULL_STATE` are both
    // CONNECTION_TYPE_REALM in legacy C++ Opcodes.cpp:1829/1832.
    host.core().send_packet_realm(&command.party_update);
    for packet in command.member_full_state_packets {
        host.core().send_raw_packet_realm(&packet);
    }
}

pub fn handle_apply_group_difficulty_command_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    command: ApplyGroupDifficultyLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }
    host.apply_group_difficulty_like_cpp(command.group_guid, command.difficulty_id, command.kind);
}

pub fn handle_apply_group_subgroup_command_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    command: ApplyGroupSubgroupLikeCppCommand,
) {
    if host.state() != SessionState::LoggedIn {
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }
    if host.resolved_group_guid_like_cpp() != Some(command.group_guid) {
        defer_group_state_reconciliation_like_cpp(host);
        return;
    }
    apply_group_subgroup_like_cpp(host, command.group_guid, command.subgroup);
}

fn apply_group_subgroup_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    group_guid: u64,
    subgroup: u8,
) {
    if host.resolved_group_guid_like_cpp() == Some(group_guid) && {
        let (state, mut hub) = host.split_social_mut();
        state.set_owned_player_group_like_cpp(&mut hub, Some((group_guid, subgroup)))
    } {
        host.sync_player_registry_state_like_cpp();
    }
}

/// #743: converge the owned group snapshot on `GroupRegistry`.
///
/// C++ never needs this: `Group::RemoveMember` (`Group.cpp:550`),
/// `Group::Disband` (`Group.cpp:713`) and `Group::ChangeMembersGroup`
/// reach every connected member's `Player` directly, so `Player::SetGroup`
/// (`Player.cpp:23440`) runs inside the same operation. RustyCore applies
/// the same transitions on the member's own session to keep its admission
/// phase, canonical Player guard and publication order; this reconciliation
/// is the fence that makes the queue hop lossless.
///
/// It runs only for a session the group authority marked, so an untouched
/// session never pays for a canonical read. A command that does arrive
/// after the reconciliation finds the snapshot already converged and its
/// own group guard rejects it, so nothing is published twice.
pub fn reconcile_group_state_like_cpp<H: GroupSessionHostLikeCpp>(host: &mut H) -> bool {
    let (Some(player_guid), Some(player_registry)) = (
        host.player_guid(),
        host.core().player_registry.as_ref().map(Arc::clone),
    ) else {
        return false;
    };
    if !player_registry.take_group_state_reconciliation_like_cpp(player_guid) {
        return false;
    }
    if host.state() != SessionState::LoggedIn {
        // The represented Player is not admitted yet. C++ resolves group
        // membership from the authority at `Player::_LoadGroup`, so the
        // mark is kept rather than discarded at an ineligible phase.
        player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
        return false;
    }
    let Some(group_registry) = host
        .core()
        .directory
        .group_registry
        .as_ref()
        .map(Arc::clone)
    else {
        return false;
    };
    let applied = apply_authoritative_group_state_like_cpp(host, player_guid, &group_registry);
    if applied == GroupReconciliationOutcomeLikeCpp::Unreachable {
        // The canonical Player could not be resolved this pass (transfer,
        // detached residence). Keep the obligation instead of losing it.
        player_registry.mark_group_state_reconciliation_like_cpp(player_guid);
        return false;
    }
    applied == GroupReconciliationOutcomeLikeCpp::Applied
}

fn defer_group_state_reconciliation_like_cpp<H: GroupSessionHostLikeCpp>(host: &H) {
    let (state, hub) = host.split_social_ref();
    state.defer_group_state_reconciliation_like_cpp(hub)
}

/// Apply the authority's membership to the owned snapshot and publish the
/// difference, in the order the original operation would have published it.
fn apply_authoritative_group_state_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    group_registry: &GroupRegistry,
) -> GroupReconciliationOutcomeLikeCpp {
    let authority = group_registry.member_group_state_like_cpp(player_guid);
    let current_group_guid = host.resolved_group_guid_like_cpp();
    let current_subgroup = {
        let (s, h) = host.split_social_ref();
        s.resolved_group_subgroup_like_cpp(h)
    };

    let Some(authority) = authority else {
        let Some(previous_group_guid) = current_group_guid else {
            return GroupReconciliationOutcomeLikeCpp::AlreadyConverged;
        };
        // `Group::Disband` retires the group itself; `Group::RemoveMember`
        // keeps it. That difference selects which teardown packet the
        // member missed.
        let surviving_category = group_registry.group_category_like_cpp(previous_group_guid);
        let category =
            surviving_category.unwrap_or(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP);
        if !{
            let (state, mut hub) = host.split_social_mut();
            state.set_owned_player_group_like_cpp(&mut hub, None)
        } {
            return GroupReconciliationOutcomeLikeCpp::Unreachable;
        }
        {
            let (state, hub) = host.split_social_ref();
            state.send_player_party_type_update_like_cpp(
                hub,
                category,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            )
        };
        host.sync_player_registry_state_like_cpp();
        let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
        if surviving_category.is_some() {
            host.core()
                .send_packet_realm(&wow_packet::packets::party::GroupUninvite);
        } else {
            host.core()
                .send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
        }
        send_destroyed_group_party_update_like_cpp(host, previous_group_guid, category);
        return GroupReconciliationOutcomeLikeCpp::Applied;
    };

    let membership_converged = current_group_guid == Some(authority.group_guid)
        && current_subgroup == Some(authority.subgroup);
    let mut applied = false;
    if !membership_converged {
        let joined_new_group = current_group_guid != Some(authority.group_guid);
        if !{
            let (state, mut hub) = host.split_social_mut();
            state.set_owned_player_group_like_cpp(
                &mut hub,
                Some((authority.group_guid, authority.subgroup)),
            )
        } {
            return GroupReconciliationOutcomeLikeCpp::Unreachable;
        }
        if joined_new_group {
            {
                let (state, hub) = host.split_social_ref();
                state.send_player_party_type_update_like_cpp(
                    hub,
                    authority.group_category,
                    wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                )
            };
            {
                let (state, hub) = host.split_social_ref();
                state.sync_player_registry_party_member_party_type_like_cpp(hub)
            };
            let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
        } else {
            host.sync_player_registry_state_like_cpp();
        }
        applied = true;
    }
    // The group also owns the member's three difficulty preferences; a lost
    // `ApplyGroupDifficultyLikeCpp` diverges them without changing membership.
    if let Some(group) = group_registry.get(&authority.group_guid)
        && reconcile_group_difficulty_like_cpp(
            host,
            group.dungeon_difficulty_id,
            group.raid_difficulty_id,
            group.legacy_raid_difficulty_id,
        )
    {
        applied = true;
    }
    if applied {
        GroupReconciliationOutcomeLikeCpp::Applied
    } else {
        GroupReconciliationOutcomeLikeCpp::AlreadyConverged
    }
}

fn send_destroyed_group_party_update_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    group_guid: u64,
    category: u8,
) {
    let (state, mut hub) = host.split_social_mut();
    state.send_destroyed_group_party_update_like_cpp(&mut hub, group_guid, category)
}

fn reconcile_group_difficulty_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    dungeon: u32,
    raid: u32,
    legacy_raid: u32,
) -> bool {
    let (state, mut hub) = host.split_instances_mut();
    state.reconcile_group_difficulty_like_cpp(&mut hub, dungeon, raid, legacy_raid)
}

/// Runs the deferred group publication tail in C++ order (#1263 F5).
///
/// The registry-state sync and the visible-gameobject refresh still need the
/// World session's stats/loot/control and map providers, so the application
/// owner returns the exact remaining sequence and the session runs it here,
/// between the canonical transition and the final packets.
pub async fn apply_group_publication_tail_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &mut H,
    tail: GroupPublicationTailLikeCpp,
) {
    use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
    match tail {
        GroupPublicationTailLikeCpp::None => {}
        GroupPublicationTailLikeCpp::VisibilityOnly => {
            let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
        GroupPublicationTailLikeCpp::PartyUpdateOnly { group } => {
            send_group_party_update_after_owner_like_cpp(host, &group);
        }
        GroupPublicationTailLikeCpp::SyncThenPartyUpdate { group } => {
            host.sync_player_registry_state_like_cpp();
            send_group_party_update_after_owner_like_cpp(host, &group);
        }
        GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupUninvite => {
            host.sync_player_registry_state_like_cpp();
            let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
            host.core()
                .send_packet_realm(&wow_packet::packets::party::GroupUninvite);
        }
        GroupPublicationTailLikeCpp::SyncThenVisibilityThenGroupDestroyed { group_guid } => {
            host.sync_player_registry_state_like_cpp();
            let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
            host.core()
                .send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
            send_destroyed_group_party_update_like_cpp(
                host,
                group_guid,
                GROUP_CATEGORY_HOME_LIKE_CPP,
            );
        }
        GroupPublicationTailLikeCpp::VisibilityThenPersistThenPartyUpdate {
            group,
            group_guid,
            persistence,
            refresh_visible_gameobjects_or_spellclicks,
        } => {
            if refresh_visible_gameobjects_or_spellclicks {
                let _ = host.update_visible_gameobjects_or_spell_clicks_like_cpp();
            }
            if !persistence.is_empty() {
                host.lifecycle()
                    .persist_group_intents_like_cpp(group_guid, persistence)
                    .await;
            }
            send_group_party_update_after_owner_like_cpp(host, &group);
        }
    }
}

fn send_group_party_update_after_owner_like_cpp<H: GroupSessionHostLikeCpp>(
    host: &H,
    group: &wow_social::group::GroupInfo,
) {
    let Some(registry) = host.core().player_registry() else {
        return;
    };
    wow_world_social::group_fanout::send_party_update(
        group,
        registry,
        host.core().virtual_realm_address(),
    );
}

/// Result of one reconciliation attempt against the group authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupReconciliationOutcomeLikeCpp {
    /// The owned snapshot already matched the authority.
    AlreadyConverged,
    /// The snapshot was corrected and the missed difference published.
    Applied,
    /// The canonical Player could not be resolved; the mark is retained.
    Unreachable,
}
