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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        if target_guid == sender_guid {
            self.apply_group_subgroup_like_cpp(group_guid, new_subgroup);
        } else if let Some(target) = registry.group_presence(target_guid) {
            let _ = registry.try_send_current_command(
                target.registration,
                SessionCommand::ApplyGroupSubgroupLikeCpp(
                    crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                        group_guid,
                        subgroup: new_subgroup,
                    },
                ),
            );
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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        for (member_guid, subgroup) in subgroup_updates {
            if member_guid == sender_guid {
                self.apply_group_subgroup_like_cpp(group_guid, subgroup);
            } else if let Some(member) = registry.group_presence(member_guid) {
                let _ = registry.try_send_current_command(
                    member.registration,
                    SessionCommand::ApplyGroupSubgroupLikeCpp(
                        crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand {
                            group_guid,
                            subgroup,
                        },
                    ),
                );
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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }
    /// CMSG_SILENCE_PARTY_TALKER.
    ///
    /// C++ parses a full `ObjectGuid Target` followed by one `Silent` bit, then
    /// returns unless the sender is in a group and is the group leader or an
    /// assistant. The live silence mutation is still a TODO in the C++ legacy
    /// source, so Rust records only the represented request at the same boundary.
    pub async fn handle_silence_party_talker(&mut self, mut pkt: wow_packet::WorldPacket) {
        let silence = match SilencePartyTalker::read(&mut pkt) {
            Ok(silence) => silence,
            Err(e) => {
                warn!("Bad SilencePartyTalker: {e}");
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

        self.record_represented_silence_party_talker_like_cpp(silence.target, silence.silent);
    }
    /// CMSG_DO_READY_CHECK.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, returns when no
    /// group exists, requires leader or assistant, then calls
    /// `Group::StartReadyCheck`. Rust represents PartyIndex over the current
    /// GroupRegistry group and approximates offline/no-session via missing
    /// PlayerRegistry entries. Timeout expiry is handled by the shared
    /// `tick_all_group_ready_checks_like_cpp` loop driven from world-server
    /// main. PartyIndex BG/BF/original-group remains a boundary if open.
    pub async fn handle_do_ready_check(&mut self, mut pkt: wow_packet::WorldPacket) {
        let ready_check = match DoReadyCheck::read(&mut pkt) {
            Ok(ready_check) => ready_check,
            Err(e) => {
                warn!("Bad DoReadyCheck: {e}");
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
    /// CMSG_READY_CHECK_RESPONSE.
    ///
    /// C++ resolves the group and calls `Group::SetMemberReadyCheck` with no
    /// leader/assistant gate. Rust preserves that represented ownership and
    /// returns with no fanout/state change when no ready check is active.
    pub async fn handle_ready_check_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        let response = match ReadyCheckResponseClient::read(&mut pkt) {
            Ok(response) => response,
            Err(e) => {
                warn!("Bad ReadyCheckResponse: {e}");
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
        let vra = self.virtual_realm_address();

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
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        send_party_update(&outcome.group, &registry, vra);
    }
    /// CMSG_SET_ROLE.
    ///
    /// C++ resolves `GetPlayer()->GetGroup(packet.PartyIndex)`, compares the
    /// target's current in-memory LFG roles, broadcasts `RoleChangedInform` to
    /// the group before `SetLfgRoles`, or sends only to the caller when no group
    /// exists. Rust represents PartyIndex as the current `GroupRegistry` group
    /// boundary and keeps `GroupInfo.member_slots.roles` as the in-memory role
    /// source of truth without DB persistence.
    pub async fn handle_set_role(&mut self, mut pkt: wow_packet::WorldPacket) {
        let set_role = match SetRole::read(&mut pkt) {
            Ok(set_role) => set_role,
            Err(e) => {
                warn!("Bad SetRole: {e}");
                return;
            }
        };
        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => {
                if set_role.role == 0 {
                    return;
                }
                self.send_packet(&RoleChangedInform {
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
            self.send_packet(&RoleChangedInform {
                party_index: GROUP_CATEGORY_HOME_LIKE_CPP,
                from: sender_guid,
                changed_unit: set_role.target_guid,
                old_role: 0,
                new_role: set_role.role,
            });
            return;
        };

        let registry = self.player_registry().map(std::sync::Arc::clone);
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
                let vra = self.virtual_realm_address();
                send_party_update(&outcome.group, registry, vra);
            }
        }
    }
    /// CMSG_UPDATE_RAID_TARGET.
    ///
    /// C++ anchor: `WorldSession::HandleUpdateRaidTargetOpcode` resolves
    /// `GetPlayer()->GetGroup(packet.PartyIndex)`. `Symbol == -1` sends only the
    /// caller a full target icon list. Other symbols call `Group::SetTargetIcon`;
    /// only raid groups gate the action to leader/assistant. Rust keeps
    /// `GroupInfo.target_icons` as canonical represented runtime state. Boundary:
    /// full ObjectAccessor/hostility checks are not represented; connected player
    /// GUID targets are accepted only when present in `PlayerRegistry`, non-player
    /// targets remain pass-through object GUIDs.
    pub async fn handle_update_raid_target(&mut self, mut pkt: wow_packet::WorldPacket) {
        let update = match UpdateRaidTarget::read(&mut pkt) {
            Ok(update) => update,
            Err(e) => {
                warn!("Bad UpdateRaidTarget: {e}");
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
                self.send_raw_packet(&raid_target_update_all_like_cpp(&group));
            }
            return;
        }

        let Ok(symbol) = u8::try_from(update.symbol) else {
            return;
        };
        let registry = match self.player_registry() {
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
    /// CMSG_REQUEST_PARTY_JOIN_UPDATES.
    ///
    /// C++ sends current target icons and raid markers for the requested party
    /// index. Rust represents raid target icons and raid marker state from
    /// `GroupInfo`.
    pub async fn handle_request_party_join_updates(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyJoinUpdates::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyJoinUpdates: {e}");
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
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            request.party_index,
        ) else {
            return;
        };
        if let Some(group) = group_reg.get(&group_guid) {
            self.send_raw_packet(&raid_target_update_all_like_cpp(&group));
            self.send_raw_packet(&raid_markers_changed_like_cpp(&group));
        }
    }
    /// CMSG_REQUEST_PARTY_MEMBER_STATS.
    ///
    /// C++ `HandleRequestPartyMemberStatsOpcode` always replies to the requester
    /// with `SMSG_PARTY_MEMBER_FULL_STATE`: `ObjectAccessor::FindConnectedPlayer`
    /// drives online/offline status. `PartyIndex` is parsed by the packet layer in
    /// the same bit/GUID/index order as C++, but the C++ handler ignores it.
    pub async fn handle_request_party_member_stats(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestPartyMemberStats::read(&mut pkt) {
            Ok(request) => request,
            Err(e) => {
                warn!("Bad RequestPartyMemberStats: {e}");
                return;
            }
        };

        let registry = self.player_registry().map(std::sync::Arc::clone);
        let state = party_member_full_state_like_cpp(request.target_guid, registry.as_deref());
        self.send_packet_realm(&state);
    }
    /// CMSG_INITIATE_ROLE_POLL.
    ///
    /// C++ resolves the current group, returns when sender is neither leader nor
    /// assistant, and broadcasts `RolePollInform` to the group with no state
    /// mutation. Rust keeps the same represented current-group boundary and uses
    /// connected PlayerRegistry recipients instead of full ObjectAccessor/sWorld.
    pub async fn handle_initiate_role_poll(&mut self, mut pkt: wow_packet::WorldPacket) {
        let role_poll = match InitiateRolePoll::read(&mut pkt) {
            Ok(role_poll) => role_poll,
            Err(e) => {
                warn!("Bad InitiateRolePoll: {e}");
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
    /// CMSG_SET_LOOT_METHOD.
    ///
    /// This Trinity branch parses the packet but has the entire mutation block
    /// disabled with `// not allowed to change`, so represented Rust preserves
    /// that no-op behavior.
    pub async fn handle_set_loot_method(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = SetLootMethod::read(&mut pkt) {
            warn!("Bad SetLootMethod: {e}");
        }
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
    /// CMSG_LOW_LEVEL_RAID1 — no-op, C++ only logs at DEBUG level.
    /// C++ anchor: GroupHandler.cpp:740-745
    pub async fn handle_low_level_raid1(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid1::read(&mut pkt) {
            warn!("Bad LowLevelRaid1: {e}");
            return;
        }
        if let Some(guid) = self.player_guid() {
            tracing::debug!("HandleLowLevelRaid1 - Player {:?}", guid);
        }
    }
    /// CMSG_LOW_LEVEL_RAID2 — no-op, C++ only logs at DEBUG level.
    /// C++ anchor: GroupHandler.cpp:747-751
    pub async fn handle_low_level_raid2(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(e) = LowLevelRaid2::read(&mut pkt) {
            warn!("Bad LowLevelRaid2: {e}");
            return;
        }
        if let Some(guid) = self.player_guid() {
            tracing::debug!("HandleLowLevelRaid2 - Player {:?}", guid);
        }
    }
}
