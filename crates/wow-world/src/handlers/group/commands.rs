// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Applying a committed group state change on the member it belongs to.
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
//! Moved here from `handlers/loot/handlers.rs` under #743: group command
//! application is not a loot responsibility and the loot adapter was already
//! over its physical budget.

use crate::session::mailbox::{
    ApplyGroupDifficultyLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyGroupSubgroupLikeCppCommand,
    SendPartyUpdateLikeCppCommand,
};
use crate::session::{SessionState, WorldSession};

impl WorldSession {
    pub(crate) fn handle_apply_group_removal_command_like_cpp(
        &mut self,
        command: ApplyGroupRemovalLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            // The command reached a phase that cannot apply it. C++ clears
            // `Player::m_group` inside `Group::RemoveMember`, so the change is
            // deferred to reconciliation instead of dropped (#743).
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }
        if self.resolved_group_guid_like_cpp() != Some(command.group_guid) {
            // Already in another group, or already removed. Nothing to undo,
            // but the mark keeps an unrelated pending divergence honest.
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }
        // A removal that raced a re-join into the same group must not clear a
        // membership the authority still holds.
        if self.authoritative_group_membership_like_cpp() == Some(command.group_guid) {
            return;
        }

        let _ = self.set_owned_player_group_like_cpp(None);
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);
        self.sync_player_registry_state_like_cpp();

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
        if command.send_group_destroyed {
            self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
        }
        if command.send_group_uninvite {
            self.send_packet_realm(&wow_packet::packets::party::GroupUninvite);
        }
        // C++ `Group::RemoveMember` (`Group.cpp:654-655`) and `Group::Disband`
        // (`Group.cpp:746`) both finish by sending the removed player the
        // destroyed `PartyUpdate` so its client tears down the party frames.
        if command.send_group_destroyed || command.send_group_uninvite {
            self.send_destroyed_group_party_update_like_cpp(command.group_guid, command.category);
        }
    }

    pub(crate) fn handle_apply_group_join_command_like_cpp(
        &mut self,
        command: ApplyGroupJoinLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }

        self.apply_group_join_like_cpp(command.group_guid, command.subgroup);
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
    }

    pub(crate) fn handle_send_party_update_command_like_cpp(
        &mut self,
        mut command: SendPartyUpdateLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.recipient) {
            return;
        }

        let Some(sequence_num) =
            self.next_group_update_sequence_number_like_cpp(command.party_update.party_index)
        else {
            return;
        };
        command.party_update.sequence_num = sequence_num;
        // `SMSG_PARTY_UPDATE` and `SMSG_PARTY_MEMBER_FULL_STATE` are both
        // CONNECTION_TYPE_REALM in legacy C++ Opcodes.cpp:1829/1832.
        self.send_packet_realm(&command.party_update);
        for packet in command.member_full_state_packets {
            self.send_raw_packet_realm(&packet);
        }
    }

    pub(crate) fn handle_apply_group_difficulty_command_like_cpp(
        &mut self,
        command: ApplyGroupDifficultyLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }
        self.apply_group_difficulty_like_cpp(
            command.group_guid,
            command.difficulty_id,
            command.kind,
        );
    }

    pub(crate) fn handle_apply_group_subgroup_command_like_cpp(
        &mut self,
        command: ApplyGroupSubgroupLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }
        if self.resolved_group_guid_like_cpp() != Some(command.group_guid) {
            self.defer_group_state_reconciliation_like_cpp();
            return;
        }
        self.apply_group_subgroup_like_cpp(command.group_guid, command.subgroup);
    }
}
