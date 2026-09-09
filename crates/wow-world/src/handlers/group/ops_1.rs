//! Group handlers operations, part 1 of 3.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    pub(super) async fn persist_group_intents_like_cpp(
        &self,
        group_guid: u64,
        intents: Vec<GroupPersistenceIntentLikeCpp>,
    ) {
        let Some(port) = self.represented_group_persistence_port_like_cpp() else {
            return;
        };
        let request = RepresentedGroupPersistenceRequestLikeCpp {
            commands: intents
                .into_iter()
                .map(group_persistence_command_like_cpp)
                .collect(),
            mode: RepresentedGroupPersistenceModeLikeCpp::Sequential,
        };
        let outcome = port.persist_group_commands_like_cpp(request).await;
        if !matches!(
            outcome,
            RepresentedGroupPersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            warn!(
                group_guid,
                ?outcome,
                "failed to persist represented group transition"
            );
        }
    }
    /// CMSG_PARTY_INVITE (0x3604)
    ///
    /// Parse layout from C++ `WorldPackets::Party::PartyInviteClient::Read`
    /// (`PartyPackets.cpp`):
    ///   HasBit() → has_party_index
    ///   ResetBitPos()
    ///   ReadBits(9) → name_len
    ///   ReadBits(9) → realm_len
    ///   ReadUInt32  → proposed_roles
    ///   ReadPackedGuid → target_guid
    ///   ReadString(name_len)
    ///   ReadString(realm_len)
    ///   [if has_party_index] ReadUInt8
    pub(crate) async fn handle_party_invite_with_policy_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
        policy: &GroupInvitePolicyLikeCpp,
    ) {
        info!(account = self.account_id, "handle_party_invite called");
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
        info!(account = self.account_id, target_name = %target_name, "PartyInvite parsed");

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        macro_rules! send_result {
            ($result:expr) => {
                self.send_packet_realm(&PartyCommandResult {
                    name: target_name.clone(),
                    command: 0, // Invite
                    result: $result,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
            };
        }

        // 2. Target must exist in the player registry (lookup by name — robust against GUID mismatch).
        let registry = match self.player_registry() {
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
                send_result!(party_result::BAD_PLAYER_NAME);
                return;
            }
        };
        let real_target_guid = target_snapshot.guid;

        // Don't invite yourself (compare by real GUID from registry).
        if real_target_guid == my_guid {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        // C++ `HandlePartyInviteOpcode` rejects inviting GM targets unless
        // `GM.AllowInvite` / `CONFIG_ALLOW_GM_GROUP` is enabled.
        if !policy.allow_gm_group
            && self.player_is_game_master_like_cpp() != Some(true)
            && target_snapshot.is_game_master
        {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        if !policy.allow_two_side_interaction
            && self.player_is_game_master_like_cpp() != Some(true)
            && player_team_for_race_cpp(self.player_race_like_cpp())
                != player_team_for_race_cpp(target_snapshot.race)
        {
            send_result!(party_result::WRONG_FACTION);
            return;
        }

        let (inviter_map_id, inviter_instance_id) =
            current_player_party_invite_map_instance_like_cpp(self, registry, my_guid);
        if inviter_instance_id != 0
            && target_snapshot.instance_id != 0
            && inviter_instance_id != target_snapshot.instance_id
            && inviter_map_id == target_snapshot.map_id
        {
            send_result!(party_result::TARGET_NOT_IN_INSTANCE);
            return;
        }

        if target_snapshot.instance_id != 0 {
            let Some(inviter_difficulty_id) = self.resolved_dungeon_difficulty_id_like_cpp() else {
                send_result!(party_result::IGNORING_YOU);
                return;
            };
            if target_snapshot.dungeon_difficulty_id != inviter_difficulty_id {
                send_result!(party_result::IGNORING_YOU);
                return;
            }
        }

        let social_port = self.social_persistence_port_like_cpp();
        if target_social_ignores_inviter_like_cpp(
            social_port.clone(),
            real_target_guid,
            my_guid,
            self.account_id,
        )
        .await
        {
            send_result!(party_result::IGNORING_YOU);
            return;
        }

        if u32::from(self.player_level_like_cpp()) < policy.minimum_level
            && !target_social_has_inviter_friend_like_cpp(social_port, real_target_guid, my_guid)
                .await
        {
            send_result!(party_result::INVITE_RESTRICTED);
            return;
        }

        // 3. The owner revalidates pending/group/category/capacity state and
        // records the invite as one transition.
        let pending = match self.pending_invites() {
            Some(p) => p,
            None => return,
        };

        let inviter_name = self.player_name_like_cpp().unwrap_or_default().to_string();
        let vra = self.virtual_realm_address();
        let (realm_name, realm_name_normalized) = self
            .realm_names_for_address_like_cpp(vra)
            .map(|(actual, normalized)| (actual.to_string(), normalized.to_string()))
            .unwrap_or_default();

        let group_reg = match self.group_registry() {
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
                send_result!(party_result::ALREADY_IN_GROUP);
                return;
            }
            CreateGroupInviteResultLikeCpp::TargetAlreadyGrouped => {
                send_result!(party_result::ALREADY_IN_GROUP);
                let invite = PartyInviteServer {
                    can_accept: false,
                    proposed_roles: proposed_roles as u8,
                    inviter_name: inviter_name.clone(),
                    inviter_guid: my_guid,
                    inviter_bnet_account_guid: ObjectGuid::create_global(
                        HighGuid::WowAccount,
                        0,
                        self.account_id as i64,
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
                send_result!(party_result::NOT_LEADER);
                return;
            }
            CreateGroupInviteResultLikeCpp::GroupFull => {
                send_result!(party_result::GROUP_FULL);
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
                self.account_id as i64,
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
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        // 7. Confirm back to self.
        self.send_packet_realm(&PartyCommandResult {
            name: target_name,
            command: 0,
            result: party_result::OK,
            result_data: 0,
            result_guid: ObjectGuid::EMPTY,
        });
    }
    #[cfg(test)]
    pub async fn handle_party_invite(&mut self, pkt: wow_packet::WorldPacket) {
        let policy = self.group_invite_policy_for_test_like_cpp();
        self.handle_party_invite_with_policy_like_cpp(pkt, &policy)
            .await;
    }
    /// CMSG_PARTY_INVITE_RESPONSE (0x3606)
    ///
    /// Parse layout:
    ///   HasBit() → has_party_index
    ///   HasBit() → accept
    ///   HasBit() → has_roles
    ///   [if has_party_index] ReadUInt8
    ///   [if has_roles]       ReadUInt8
    pub async fn handle_party_invite_response(&mut self, mut pkt: wow_packet::WorldPacket) {
        // — parse —
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

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };
        let my_name = self.player_name_like_cpp().unwrap_or_default().to_string();

        // Clone Arcs immediately so we hold no borrow on `self` later.
        let pending = match self.pending_invites() {
            Some(p) => std::sync::Arc::clone(p),
            None => return,
        };

        // 1. Must have a pending C++ `GroupInvite`.
        let invite = match pending.get(&my_guid) {
            Some(invite) => invite,
            None => return,
        };

        let registry = match self.player_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };

        let group_reg = match self.group_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        // 2. Declined?
        if !accept {
            let Some(invite) = group_reg.decline_invite_like_cpp(&pending, my_guid, party_index)
            else {
                return;
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
            return;
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
            | AcceptGroupInviteResultLikeCpp::MissingLeader => return,
            AcceptGroupInviteResultLikeCpp::SelfInvite => {
                warn!(
                    player = %my_guid,
                    "HandlePartyInviteResponse: player tried to accept an invite to his own group"
                );
                return;
            }
            AcceptGroupInviteResultLikeCpp::GroupFull => {
                self.send_packet_realm(&PartyCommandResult {
                    name: String::new(),
                    command: 0,
                    result: party_result::GROUP_FULL,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
                return;
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
                    let _ = registry.try_send_current_command(
                        leader.registration,
                        SessionCommand::ApplyGroupJoinLikeCpp(ApplyGroupJoinLikeCppCommand {
                            group_guid: group.group_guid,
                            category: group.group_category_like_cpp(),
                            party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                            subgroup: 0,
                            refresh_visible_gameobjects_or_spellclicks: false,
                        }),
                    );
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
            self.apply_group_join_like_cpp(group_guid, subgroup);
        }
        if let Some(group) = group_reg.get(&group_guid) {
            self.send_player_party_type_update_like_cpp(
                group.group_category_like_cpp(),
                wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
            );
        }
        self.sync_player_registry_party_member_party_type_like_cpp();
        if refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }

        if !persistence.is_empty() {
            self.persist_group_intents_like_cpp(group_guid, persistence)
                .await;
        }

        // 4. Send PartyUpdate + PartyMemberFullState to all members.
        let vra = self.virtual_realm_address();
        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
    }
    /// CMSG_PARTY_UNINVITE.
    ///
    /// C++ `WorldPackets::Party::PartyUninvite::Read` reads an optional
    /// party-index bit, an 8-bit reason length, target GUID, optional party
    /// index, then the reason string. `HandlePartyUninviteOpcode` rejects self,
    /// checks `CanUninviteFromGroup`, and calls
    /// `Player::RemoveFromGroup(... GROUP_REMOVEMETHOD_KICK ...)` when the
    /// target is a current member.
    pub async fn handle_party_uninvite(&mut self, mut pkt: wow_packet::WorldPacket) {
        let uninvite = match wow_packet::packets::party::PartyUninvite::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!("Bad PartyUninvite: {error}");
                return;
            }
        };

        let sender_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };
        if uninvite.target_guid == sender_guid {
            return;
        }

        let group_reg = match self.group_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => {
                send_party_uninvite_result_like_cpp(self, party_result::NOT_IN_GROUP);
                return;
            }
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let pending_invites = self.pending_invites().map(std::sync::Arc::clone);
        let Some(group_guid) = current_group_guid_like_cpp(
            &group_reg,
            self.resolved_group_guid_like_cpp(),
            sender_guid,
            uninvite.party_index,
        ) else {
            send_party_uninvite_result_like_cpp(self, party_result::NOT_IN_GROUP);
            return;
        };

        let Some(group_snapshot) = group_reg.get(&group_guid) else {
            return;
        };
        let target_has_loot_rolls = registry
            .group_presence(uninvite.target_guid)
            .is_some_and(|target| target.has_active_loot_rolls);
        let sender_map_id = self.player_map_id_like_cpp();
        let sender_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let any_member_in_combat = group_snapshot.members.iter().any(|member_guid| {
            if *member_guid == sender_guid {
                self.resolved_in_combat_like_cpp() != Some(false)
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
                actor_in_battleground: self.player_in_represented_battleground_like_cpp(),
                target_has_loot_rolls,
                any_member_in_actor_map_combat: any_member_in_combat,
            },
            &[],
        ) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::LfgBootLimit) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_LIMIT);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootTooFewPlayers) => {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::PARTY_LFG_BOOT_TOO_FEW_PLAYERS,
                );
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootDungeonComplete) => {
                send_party_uninvite_result_like_cpp(
                    self,
                    party_result::PARTY_LFG_BOOT_DUNGEON_COMPLETE,
                );
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootLootRolls) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_LOOT_ROLLS);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgBootInCombat) => {
                send_party_uninvite_result_like_cpp(self, party_result::PARTY_LFG_BOOT_IN_COMBAT);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::NotLeaderOrAssistant)
            | Err(GroupAuthorityErrorLikeCpp::TargetIsLeader) => {
                send_party_uninvite_result_like_cpp(self, party_result::NOT_LEADER);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::InviteRestricted) => {
                send_party_uninvite_result_like_cpp(self, party_result::INVITE_RESTRICTED);
                return;
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
                        return;
                    }
                }
                send_party_uninvite_result_like_cpp(self, party_result::TARGET_NOT_IN_GROUP);
                return;
            }
            Err(GroupAuthorityErrorLikeCpp::LfgKickOwnedByVote) => return,
            Err(_) => return,
        };
        let should_disband = outcome.facts.disbanded;
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
            .await;

        let cleanup_command = ApplyGroupRemovalLikeCppCommand {
            group_guid,
            category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            send_group_destroyed: should_disband,
            send_group_uninvite: !should_disband,
            refresh_visible_gameobjects_or_spellclicks: true,
        };
        if let Some(target) = registry.group_presence(uninvite.target_guid) {
            let _ = registry.try_send_current_command(
                target.registration,
                SessionCommand::ApplyGroupRemovalLikeCpp(cleanup_command),
            );
        }

        if should_disband {
            let _ = self.set_owned_player_group_like_cpp(None);
            self.clear_represented_group_subgroup_like_cpp();
            self.send_player_party_type_update_like_cpp(
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
            // C++ `Group::Disband` sends every member the destroyed
            // `PartyUpdate` after `GroupDestroyed` (`Group.cpp:744-746`).
            self.send_destroyed_group_party_update_like_cpp(
                group_guid,
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            );
            return;
        }

        send_party_update(&outcome.group, &registry, self.virtual_realm_address());
    }
    /// CMSG_LEAVE_GROUP (0x364c)
    ///
    /// Parse layout:
    ///   HasBit() → has_party_index
    ///   [if has_party_index] ReadUInt8
    pub async fn handle_leave_group(&mut self, mut pkt: wow_packet::WorldPacket) {
        // — parse —
        let has_party_index = pkt.read_bit().unwrap_or(false);
        let party_index = if has_party_index {
            pkt.read_uint8().ok()
        } else {
            None
        };

        // — setup —
        let my_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        // Clone Arcs immediately so we hold no borrow on `self` during mutations.
        let group_reg = match self.group_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        let registry = match self.player_registry() {
            Some(r) => std::sync::Arc::clone(r),
            None => return,
        };
        let pending_invites = self.pending_invites().map(std::sync::Arc::clone);
        let vra = self.virtual_realm_address();

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
            return;
        };

        if self.player_in_represented_battleground_like_cpp() {
            self.send_packet_realm(&PartyCommandResult {
                name: String::new(),
                command: 0,
                result: party_result::INVITE_RESTRICTED,
                result_data: 0,
                result_guid: ObjectGuid::EMPTY,
            });
            return;
        }

        let player_name = self.player_name_like_cpp().unwrap_or_default().to_string();

        if real_group_guid.is_none() {
            if let (Some(pending_invites), Some(invite)) =
                (pending_invites.as_ref(), pending_invite)
            {
                if invite.leader_guid == my_guid {
                    self.send_packet_realm(&PartyCommandResult {
                        name: player_name,
                        command: 2,
                        result: party_result::OK,
                        result_data: 0,
                        result_guid: ObjectGuid::EMPTY,
                    });
                    group_reg.cancel_pending_group_like_cpp(pending_invites, invite);
                }
            }
            return;
        }
        let gid = real_group_guid.expect("checked above");

        self.send_packet_realm(&PartyCommandResult {
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
            Err(_) => return,
        };
        let dissolve_remaining = outcome
            .facts
            .disbanded
            .then(|| outcome.facts.remaining_members.clone());
        self.persist_group_intents_like_cpp(gid, outcome.persistence)
            .await;

        if let Some(remaining) = dissolve_remaining {
            // Group dissolved — notify last remaining member (if any).
            if let Some(&last_guid) = remaining.first() {
                if let Some(last) = registry.group_presence(last_guid) {
                    let command = ApplyGroupRemovalLikeCppCommand {
                        group_guid: gid,
                        category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                        send_group_destroyed: true,
                        send_group_uninvite: false,
                        refresh_visible_gameobjects_or_spellclicks: true,
                    };
                    let _ = registry.try_send_current_command(
                        last.registration,
                        SessionCommand::ApplyGroupRemovalLikeCpp(command),
                    );
                }
            }
            // Tell self to leave.
            let _ = self.set_owned_player_group_like_cpp(None);
            self.clear_represented_group_subgroup_like_cpp();
            self.send_player_party_type_update_like_cpp(
                wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
            );
            self.sync_player_registry_state_like_cpp();
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
            self.send_packet_realm(&GroupUninvite);
            return;
        }

        // 3. Send updated PartyUpdate to remaining members.
        send_party_update(&outcome.group, &registry, vra);

        // 4. Uninvite self.
        let _ = self.set_owned_player_group_like_cpp(None);
        self.clear_represented_group_subgroup_like_cpp();
        self.send_player_party_type_update_like_cpp(
            wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
            wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
        );
        self.sync_player_registry_state_like_cpp();
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        self.send_packet_realm(&GroupUninvite);
    }
    /// CMSG_CONVERT_RAID.
    ///
    /// C++ `WorldPackets::Party::ConvertRaid::Read` reads a single `Raid` bit.
    pub async fn handle_convert_raid(&mut self, mut pkt: wow_packet::WorldPacket) {
        let convert = match wow_packet::packets::party::ConvertRaid::read(&mut pkt) {
            Ok(convert) => convert,
            Err(e) => {
                warn!("Bad ConvertRaid: {e}");
                return;
            }
        };

        let my_guid = match self.player_guid() {
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
            my_guid,
            None,
        ) else {
            return;
        };
        let registry = match self.player_registry() {
            Some(registry) => std::sync::Arc::clone(registry),
            None => return,
        };
        let vra = self.virtual_realm_address();

        let outcome = match group_reg.convert_group_like_cpp(group_guid, my_guid, convert.raid) {
            Ok(outcome) => outcome,
            Err(GroupAuthorityErrorLikeCpp::GroupTooLarge) => {
                self.send_packet_realm(&PartyCommandResult {
                    name: String::new(),
                    command: 0,
                    result: party_result::OK,
                    result_data: 0,
                    result_guid: ObjectGuid::EMPTY,
                });
                return;
            }
            Err(_) => return,
        };
        self.send_packet_realm(&PartyCommandResult {
            name: String::new(),
            command: 0,
            result: party_result::OK,
            result_data: 0,
            result_guid: ObjectGuid::EMPTY,
        });
        self.persist_group_intents_like_cpp(group_guid, outcome.persistence)
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
        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }
}
