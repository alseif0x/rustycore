//! Group handlers operations, part 1 of 3.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
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
        info!(account = self.core.account_id, "handle_party_invite called");
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
        info!(account = self.core.account_id, target_name = %target_name, "PartyInvite parsed");

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
            && crate::session::hub_ref(self).player_is_game_master_like_cpp() != Some(true)
            && target_snapshot.is_game_master
        {
            send_result!(party_result::BAD_PLAYER_NAME);
            return;
        }

        if !policy.allow_two_side_interaction
            && crate::session::hub_ref(self).player_is_game_master_like_cpp() != Some(true)
            && player_team_for_race_cpp(crate::session::hub_ref(self).player_race_like_cpp())
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

        let social_port = self.lifecycle.social_persistence_port_like_cpp();
        if target_social_ignores_inviter_like_cpp(
            social_port.clone(),
            real_target_guid,
            my_guid,
            self.core.account_id,
        )
        .await
        {
            send_result!(party_result::IGNORING_YOU);
            return;
        }

        if u32::from(crate::session::hub_ref(self).player_level_like_cpp()) < policy.minimum_level
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

        let inviter_name = crate::session::hub_ref(self)
            .player_name_like_cpp()
            .unwrap_or_default();
        let vra = self.core.virtual_realm_address();
        let (realm_name, realm_name_normalized) = self
            .core
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
                        self.core.account_id as i64,
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
                self.core.account_id as i64,
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
        let my_name = crate::session::hub_ref(self)
            .player_name_like_cpp()
            .unwrap_or_default();

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
                    let _ = registry.deliver_group_state_command_like_cpp(
                        leader.registration,
                        SessionCommand::ApplyGroupJoinLikeCpp(ApplyGroupJoinLikeCppCommand {
                            group_guid: group.group_guid,
                            category: group.group_category_like_cpp(),
                            party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
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
            {
                let (s, mut h) = crate::session::split_social_mut(self);
                s.apply_group_join_like_cpp(&mut h, group_guid, subgroup)
            };
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
            self.lifecycle
                .persist_group_intents_like_cpp(group_guid, persistence)
                .await;
        }

        // 4. Send PartyUpdate + PartyMemberFullState to all members.
        let vra = self.core.virtual_realm_address();
        if let Some(group) = group_reg.get(&group_guid) {
            send_party_update(&group, &registry, vra);
        }
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/group/ops_1/f3_shims.rs"]
mod f3_shims;
