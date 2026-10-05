//! Group handlers operations, part 2 of 3.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    /// CMSG_CHANGE_SUB_GROUP.
    ///
    /// C++ `WorldPackets::Party::ChangeSubGroup::Read` reads target GUID,
    /// target subgroup, then an optional party index bit/value.
    pub async fn handle_change_sub_group(&mut self, mut pkt: wow_packet::WorldPacket) {
        let change = match wow_packet::packets::party::ChangeSubGroup::read(&mut pkt) {
            Ok(change) => change,
            Err(e) => {
                warn!("Bad ChangeSubGroup: {e}");
                return;
            }
        };

        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        if usize::from(change.new_subgroup) >= wow_social::group::MAX_RAID_SUBGROUPS_LIKE_CPP {
            return;
        }

        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            change.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.change_member_subgroup_like_cpp(
            group_guid,
            sender_guid,
            change.target_guid,
            change.new_subgroup,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let (target_guid, new_subgroup) = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        if target_guid == sender_guid {
            self.apply_group_subgroup_like_cpp(group_guid, new_subgroup);
        } else if let Some(target) = registry.group_presence(target_guid) {
            let _ = registry.deliver_group_state_command_like_cpp(
                target.registration,
                SessionCommand::ApplyGroupSubgroupLikeCpp(
                    crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                        group_guid,
                        subgroup: new_subgroup,
                    },
                ),
            );
        } else {
            // C++ `Group::ChangeMembersGroup` sets the member's subgroup on the
            // member itself; an unresolvable member reconciles later (#743).
            registry.mark_group_state_reconciliation_like_cpp(target_guid);
        }

        send_party_update(&outcome.group, &registry, vra);
    }
    /// CMSG_SWAP_SUB_GROUPS.
    ///
    /// C++ `WorldPackets::Party::SwapSubGroups::Read` reads the optional
    /// party-index bit first, then first/second target GUIDs, then `PartyIndex`
    /// when present. `PartyIndex` is parsed but remains a represented boundary
    /// here: BG/BF/original-group selection is not full parity yet. The bounded
    /// source of truth is the represented `GroupRegistry` state; if a character
    /// DB is attached, the two C++ subgroup update statements are executed in
    /// order after the registry mutation. C++ wraps those statements in a
    /// transaction; Rust does not have real transaction/rollback parity yet.
    pub async fn handle_swap_sub_groups(&mut self, mut pkt: wow_packet::WorldPacket) {
        let swap = match wow_packet::packets::party::SwapSubGroups::read(&mut pkt) {
            Ok(swap) => swap,
            Err(e) => {
                warn!("Bad SwapSubGroups: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.core.virtual_realm_address();

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            swap.party_index,
        ) else {
            return;
        };

        let outcome = match group_reg.swap_member_subgroups_like_cpp(
            group_guid,
            sender_guid,
            swap.first_target,
            swap.second_target,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let subgroup_updates = outcome.facts;
        self.lifecycle
            .persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        for (member_guid, subgroup) in subgroup_updates {
            if member_guid == sender_guid {
                self.apply_group_subgroup_like_cpp(group_guid, subgroup);
            } else if let Some(member) = registry.group_presence(member_guid) {
                let _ = registry.deliver_group_state_command_like_cpp(
                    member.registration,
                    SessionCommand::ApplyGroupSubgroupLikeCpp(
                        crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                            group_guid,
                            subgroup,
                        },
                    ),
                );
            } else {
                registry.mark_group_state_reconciliation_like_cpp(member_guid);
            }
        }

        send_party_update(&outcome.group, &registry, vra);
    }
    /// CMSG_SET_PARTY_LEADER.
    ///
    /// C++ resolves `ObjectAccessor::FindConnectedPlayer(packet.TargetGUID)`,
    /// gets `GetPlayer()->GetGroup(packet.PartyIndex)`, requires the sender to
    /// be current leader and the target to belong to that same group, then
    /// calls `Group::ChangeLeader` followed by `Group::SendUpdate`.
    ///
    /// Rust preserves the represented state transitions available today:
    /// connected target gate via `PlayerRegistry`, member gate via
    /// `GroupRegistry`, leader mutation, assistant flag removal for the new
    /// leader, optional DB persistence, `GroupNewLeader`, and `PartyUpdate`.
    /// Player flag/name/faction/script side effects remain represented
    /// boundaries until live player objects own those fields.
    pub async fn handle_set_party_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_leader = match SetPartyLeader::read(&mut pkt) {
            Ok(set_leader) => set_leader,
            Err(e) => {
                warn!("Bad SetPartyLeader: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let Some(target) = registry.social_recipient(set_leader.target_guid) else {
            return;
        };
        let target_name = target.player_name;
        let vra = self.core.virtual_realm_address();

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
    /// CMSG_SET_ASSISTANT_LEADER.
    ///
    /// C++ reads has-party-index bit, apply bit, target GUID and optional
    /// PartyIndex, then resolves `GetPlayer()->GetGroup(packet.PartyIndex)`.
    /// Rust parses PartyIndex but keeps BG/BF/original-group selection as a
    /// represented boundary; source of truth is the current `GroupRegistry`
    /// group. Registry mutation happens before optional CharacterDB persistence
    /// and PartyUpdate fanout, and no await is performed while holding the
    /// mutable group guard.
    pub async fn handle_set_assistant_leader(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_assistant = match SetAssistantLeader::read(&mut pkt) {
            Ok(set_assistant) => set_assistant,
            Err(e) => {
                warn!("Bad SetAssistantLeader: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.core.virtual_realm_address();

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
    /// CMSG_SET_EVERYONE_IS_ASSISTANT.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, rejects missing
    /// group and non-leader senders, then calls `Group::SetEveryoneIsAssistant`.
    /// Rust parses PartyIndex but keeps BG/BF/original-group selection as a
    /// represented boundary over the current `GroupRegistry` group.
    pub async fn handle_set_everyone_is_assistant(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_everyone = match SetEveryoneIsAssistant::read(&mut pkt) {
            Ok(set_everyone) => set_everyone,
            Err(e) => {
                warn!("Bad SetEveryoneIsAssistant: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.core.virtual_realm_address();

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
    /// CMSG_SET_PARTY_ASSIGNMENT.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, requires leader
    /// or raid assistant, maps `GROUP_ASSIGN_MAINTANK`/`GROUP_ASSIGN_MAINASSIST`
    /// to the corresponding unique member flag, calls `RemoveUniqueGroupMemberFlag`
    /// before attempting `SetGroupMemberFlag`, then calls `Group::SendUpdate`
    /// after the switch. Rust keeps PartyIndex as a represented boundary over
    /// the current `GroupRegistry` group; represented unique clears are live
    /// in-memory only, and DB persistence is limited to the target row returned
    /// by the C++-like `SetGroupMemberFlag` path before PartyUpdate fanout.
    pub async fn handle_set_party_assignment(&mut self, mut pkt: wow_packet::WorldPacket) {
        let assignment = match SetPartyAssignment::read(&mut pkt) {
            Ok(assignment) => assignment,
            Err(e) => {
                warn!("Bad SetPartyAssignment: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.core.virtual_realm_address();

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
    /// CMSG_CLEAR_RAID_MARKER.
    ///
    /// C++ `WorldSession::HandleClearRaidMarker` resolves the player's current
    /// HOME group, gates raid groups to leader/assistant, then calls
    /// `Group::DeleteRaidMarker`. Marker id `8` is the C++ "clear all" sentinel.
    pub async fn handle_clear_raid_marker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let clear = match ClearRaidMarker::read(&mut pkt) {
            Ok(clear) => clear,
            Err(e) => {
                warn!("Bad ClearRaidMarker: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let registry = match self.player_registry() {
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

        let outcome = match group_reg.delete_raid_marker_transition_like_cpp(
            group_guid,
            sender_guid,
            clear.marker_id,
        ) {
            Ok(outcome) => outcome,
            Err(_) => return,
        };
        let bytes = raid_markers_changed_like_cpp(&outcome.group);
        let recipients = connected_group_member_txs_like_cpp(&outcome.group, &registry);

        send_group_packet_bytes_like_cpp(&registry, bytes, &recipients);
    }
    /// CMSG_OPT_OUT_OF_LOOT — toggle automatic pass on group-loot rolls.
    pub async fn handle_opt_out_of_loot(&mut self, mut pkt: wow_packet::WorldPacket) {
        let opt_out = match OptOutOfLoot::read(&mut pkt) {
            Ok(opt_out) => opt_out,
            Err(e) => {
                warn!("Bad OptOutOfLoot: {e}");
                return;
            }
        };

        if self.player_guid().is_none() {
            if opt_out.pass_on_loot {
                warn!("CMSG_OPT_OUT_OF_LOOT value<>0 for not-loaded character");
            }
            return;
        }

        let _ = self.set_pass_on_group_loot_like_cpp(opt_out.pass_on_loot);
    }

    pub async fn handle_random_roll(&mut self, mut pkt: wow_packet::WorldPacket) {
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

        let Some(sender_guid) = self.player_guid() else {
            return;
        };

        let result = rand::thread_rng().gen_range(roll.min..=roll.max);
        let response = RandomRoll {
            roller: sender_guid,
            roller_wow_account: ObjectGuid::new(
                (HighGuid::WowAccount as i64) << 58,
                i64::from(self.core.account_id),
            ),
            min: roll.min,
            max: roll.max,
            result,
        };
        let bytes = response.to_bytes();

        let Some(group_reg) = self.group_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
            return;
        };

        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            None,
        ) else {
            self.send_packet(&response);
            return;
        };

        let Some(group) = group_reg.get(&group_guid) else {
            self.send_packet(&response);
            return;
        };

        let Some(registry) = self.player_registry().map(std::sync::Arc::clone) else {
            self.send_packet(&response);
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
            self.send_packet(&response);
        }
    }
}
