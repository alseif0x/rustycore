//! Group handlers state definitions, part 1 of 1.
//!
//! Separated from the group.rs root under #662. Behaviour is preserved.

use super::*;

pub(super) const PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP: Duration = Duration::from_millis(250);

/// Canonical represented group lookup matching C++ `Player::GetGroup` semantics.
///
/// C++ anchor: `Player::GetGroup(Optional<uint8> partyIndex)` at
/// `/home/server/woltk-trinity-legacy/src/server/game/Entities/Player/Player.cpp:23429-23444`.
///
/// 1. Validates `cached_group_guid` against canonical `GroupRegistry` membership
///    and represented `PartyIndex`/`GroupCategory`: the cached group must exist,
///    `sender_guid` must be a current member, and the represented category must
///    match when `party_index` is present.
/// 2. If cache is missing, stale, or category-mismatched, scans `GroupRegistry`
///    for a group containing `sender_guid` that also matches `party_index`.
/// 3. Returns `None` when `sender_guid` is not a member of any represented group
///    matching the requested category.
///
/// Boundary: RustyCore currently represents HOME groups only by default.
/// `PartyIndex=Some(1)` / INSTANCE, original-group, BG and BF group ownership do
/// not fall back to HOME and remain unsupported until real state exists.
pub(super) fn current_group_guid_like_cpp(
    group_reg: &GroupRegistry,
    cached_group_guid: Option<u64>,
    sender_guid: ObjectGuid,
    party_index: Option<u8>,
) -> Option<u64> {
    // 1. Validate cache: group must exist, sender must be a member, and category must match.
    if let Some(gid) = cached_group_guid {
        if let Some(group) = group_reg.get(&gid) {
            if group.members.contains(&sender_guid)
                && group.matches_party_index_like_cpp(party_index)
            {
                return Some(gid);
            }
        }
    }
    // 2. Fallback: scan for any group containing sender in the requested category.
    group_reg
        .snapshots()
        .into_iter()
        .find(|group| {
            group.members.contains(&sender_guid) && group.matches_party_index_like_cpp(party_index)
        })
        .map(|group| group.group_guid)
}

pub(super) fn party_member_full_state_like_cpp(
    target_guid: ObjectGuid,
    registry: Option<&PlayerRegistry>,
) -> PartyMemberFullState {
    let Some(entry) = registry.and_then(|registry| registry.party_member(target_guid)) else {
        return PartyMemberFullState {
            member_guid: target_guid,
            for_enemy: false,
            status: 0,
            power_type: 0,
            current_health: 0,
            max_health: 0,
            current_power: 0,
            max_power: 0,
            level: 0,
            spec_id: 0,
            zone_id: 0,
            position_x: 0,
            position_y: 0,
            position_z: 0,
            vehicle_seat: 0,
            party_type: [0; 2],
            phases: Default::default(),
            auras: Vec::new(),
            pet_stats: None,
            dungeon_score: Default::default(),
        };
    };

    let pos = entry.position;
    // Represented subset of C++ `PartyMemberFullState::Initialize(Player*)`.
    // Remaining unsupported runtime-owned fields stay explicit instead of
    // being guessed here.
    let mut status = 1u16; // MEMBER_STATUS_ONLINE
    if entry.is_pvp {
        status |= 0x0002; // MEMBER_STATUS_PVP
    }
    if !entry.is_alive {
        if entry.is_ghost {
            status |= 0x0008; // MEMBER_STATUS_GHOST
        } else {
            status |= 0x0004; // MEMBER_STATUS_DEAD
        }
    }
    if entry.is_ffa_pvp {
        status |= 0x0010; // MEMBER_STATUS_PVP_FFA
    }
    if entry.is_afk {
        status |= 0x0040; // MEMBER_STATUS_AFK
    }
    if entry.is_dnd {
        status |= 0x0080; // MEMBER_STATUS_DND
    }
    if entry.in_vehicle {
        status |= 0x0200; // MEMBER_STATUS_VEHICLE
    }

    PartyMemberFullState {
        member_guid: target_guid,
        for_enemy: false,
        status,
        power_type: entry.power_type,
        current_health: i32::try_from(entry.current_health).unwrap_or(i32::MAX),
        max_health: i32::try_from(entry.max_health).unwrap_or(i32::MAX),
        current_power: entry.current_power,
        max_power: entry.max_power,
        level: entry.level as u16,
        spec_id: entry.spec_id.min(u32::from(u16::MAX)) as u16,
        zone_id: entry.zone_id.min(u32::from(u16::MAX)) as u16,
        position_x: pos.x as i16,
        position_y: pos.y as i16,
        position_z: pos.z as i16,
        vehicle_seat: entry.party_member_vehicle_seat,
        party_type: entry.party_member_party_type,
        phases: entry.party_member_phase_states.clone(),
        auras: entry.party_member_auras.clone(),
        pet_stats: entry.party_member_pet_stats.clone(),
        dungeon_score: Default::default(),
    }
}

pub(super) fn party_player_info_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    guid: ObjectGuid,
) -> Option<PartyPlayerInfo> {
    let slot = group.member_slot_like_cpp(guid);
    registry.social_recipient(guid).map(|entry| {
        let race = if entry.race == 0 {
            slot.map(|slot| slot.race).unwrap_or_default()
        } else {
            entry.race
        };
        PartyPlayerInfo {
            guid,
            name: if entry.player_name.is_empty() {
                slot.map(|slot| slot.name.clone()).unwrap_or_default()
            } else {
                entry.player_name.clone()
            },
            class: if entry.class == 0 {
                slot.map(|slot| slot.class).unwrap_or_default()
            } else {
                entry.class
            },
            subgroup: slot.map(|slot| slot.subgroup).unwrap_or_default(),
            flags: slot.map(|slot| slot.flags).unwrap_or_default(),
            roles_assigned: slot.map(|slot| slot.roles).unwrap_or_default(),
            faction_group: if race <= 5 { 1 } else { 2 },
            connected: true,
        }
    })
}

/// Sends `PartyUpdate` + `PartyMemberFullState` to every member of `group`.
///
/// Each member gets a `PartyUpdate` where their own `my_index` reflects their
/// position in the member list.  A `PartyMemberFullState` is then sent for
/// every *other* member.
pub(super) fn send_party_update(group: &GroupInfo, registry: &PlayerRegistry, _vra: u32) {
    // Pre-build the full PlayerList (ALL members including each receiver)
    let all_players: Vec<PartyPlayerInfo> = group
        .members
        .iter()
        .filter_map(|&guid| party_player_info_like_cpp(group, registry, guid))
        .collect();

    for (my_idx, &member_guid) in group.members.iter().enumerate() {
        let member_entry = match registry.control_address(member_guid) {
            Some(e) => e,
            None => continue,
        };

        let update = PartyUpdate {
            party_flags: group.group_flags,
            party_index: group.group_category_like_cpp(),
            party_type: 1,
            my_index: my_idx as i32,
            party_guid: group.group_guid,
            // Filled by the receiver's WorldSession from its per-player
            // `NextGroupUpdateSequenceNumber` state.
            sequence_num: 0,
            leader_guid: group.leader_guid,
            leader_faction_group: 0,
            player_list: all_players.clone(), // ALL members, receiver included
            loot_settings: Some(PartyLootSettings {
                method: group.loot_method,
                loot_master: if group.loot_method == 2 {
                    group.master_looter_guid
                } else {
                    ObjectGuid::EMPTY
                },
                threshold: group.loot_threshold,
            }),
            difficulty_settings: Some(PartyDifficultySettings {
                dungeon_difficulty_id: group.dungeon_difficulty_id,
                raid_difficulty_id: group.raid_difficulty_id,
                legacy_raid_difficulty_id: group.legacy_raid_difficulty_id,
            }),
        };

        let mut member_full_state_packets = Vec::new();
        for &other_guid in &group.members {
            if other_guid == member_guid {
                continue;
            }
            if registry.party_member(other_guid).is_some() {
                let full_state = party_member_full_state_like_cpp(other_guid, Some(registry));
                member_full_state_packets.push(full_state.to_bytes());
            }
        }

        let command = SendPartyUpdateLikeCppCommand {
            recipient: member_guid,
            party_update: update,
            member_full_state_packets,
        };
        #[cfg(not(test))]
        if registry
            .try_send_current_command(
                member_entry.registration(),
                SessionCommand::SendPartyUpdateLikeCpp(command),
            )
            .is_err()
        {
            warn!(member = %member_guid, "failed to queue party update for remote session");
        }
        #[cfg(test)]
        {
            registry
                .send_current_command_blocking_timeout(
                    member_entry.registration(),
                    SessionCommand::SendPartyUpdateLikeCpp(command),
                    Duration::from_secs(1),
                )
                .expect("test session dispatcher accepts PartyUpdate command");
        }
    }
}

pub(super) async fn send_group_new_leader_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    new_leader_name: &str,
) {
    let packet = GroupNewLeader {
        party_index: group.group_category_like_cpp() as i8,
        name: new_leader_name.to_string(),
    }
    .to_bytes();

    let recipients = registry.group_presences_in_order(&group.members);
    for recipient in recipients {
        let _ = send_realm_packet_to_player_like_cpp(
            registry,
            recipient.registration,
            recipient.guid,
            packet.clone(),
        )
        .await;
    }
}

pub(super) fn first_connected_group_member_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Option<ObjectGuid> {
    group
        .members
        .iter()
        .copied()
        .find(|member_guid| registry.group_presence(*member_guid).is_some())
}

pub(super) fn sender_can_start_ready_check_like_cpp(
    group: &GroupInfo,
    sender_guid: ObjectGuid,
) -> bool {
    group.leader_guid == sender_guid
        || group
            .member_slot_like_cpp(sender_guid)
            .is_some_and(|slot| (slot.flags & MEMBER_FLAG_ASSISTANT_LIKE_CPP) != 0)
}

pub(super) fn current_player_party_invite_map_instance_like_cpp(
    session: &WorldSession,
    registry: &PlayerRegistry,
    player_guid: ObjectGuid,
) -> (u16, u32) {
    if let Some(key) = session.current_canonical_player_map_key_like_cpp() {
        return (key.map_id.min(u32::from(u16::MAX)) as u16, key.instance_id);
    }

    registry
        .group_presence(player_guid)
        .map(|entry| (entry.map_id, entry.instance_id))
        .unwrap_or_else(|| (session.player_map_id_like_cpp(), 0))
}

pub(super) async fn target_social_ignores_inviter_like_cpp(
    port: Option<std::sync::Arc<dyn SocialPersistencePortLikeCpp>>,
    target_guid: ObjectGuid,
    inviter_guid: ObjectGuid,
    inviter_account_id: u32,
) -> bool {
    let Some(port) = port else {
        return false;
    };

    match port
        .party_invite_target_ignores_like_cpp(
            target_guid.counter(),
            inviter_guid.counter(),
            inviter_account_id,
        )
        .await
    {
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(ignores) => ignores,
        SocialPartyInviteLookupOutcomeLikeCpp::Failed { reason } => {
            warn!(
                error = %reason,
                target = ?target_guid,
                inviter = ?inviter_guid,
                "PartyInvite social ignore lookup failed"
            );
            false
        }
    }
}

pub(super) async fn target_social_has_inviter_friend_like_cpp(
    port: Option<std::sync::Arc<dyn SocialPersistencePortLikeCpp>>,
    target_guid: ObjectGuid,
    inviter_guid: ObjectGuid,
) -> bool {
    let Some(port) = port else {
        return false;
    };

    match port
        .party_invite_target_has_friend_like_cpp(target_guid.counter(), inviter_guid.counter())
        .await
    {
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(has_friend) => has_friend,
        SocialPartyInviteLookupOutcomeLikeCpp::Failed { reason } => {
            warn!(
                error = %reason,
                target = ?target_guid,
                inviter = ?inviter_guid,
                "PartyInvite social friend lookup failed"
            );
            false
        }
    }
}

pub(super) fn connected_group_members_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<ObjectGuid> {
    group
        .members
        .iter()
        .copied()
        .filter(|member_guid| registry.group_presence(*member_guid).is_some())
        .collect()
}

pub(super) fn send_ready_check_events_like_cpp(
    events: &[ReadyCheckEventLikeCpp],
    group: &GroupInfo,
    registry: &PlayerRegistry,
) {
    let recipients = registry.group_presences_in_order(&group.members);

    for event in events {
        let bytes = match *event {
            ReadyCheckEventLikeCpp::Started {
                party_index,
                party_guid,
                initiator_guid,
                duration_ms,
            } => ReadyCheckStarted {
                party_index,
                party_guid,
                initiator_guid,
                duration_ms,
            }
            .to_bytes(),
            ReadyCheckEventLikeCpp::Response {
                party_guid,
                player,
                is_ready,
            } => ReadyCheckResponse {
                party_guid,
                player,
                is_ready,
            }
            .to_bytes(),
            ReadyCheckEventLikeCpp::Completed {
                party_index,
                party_guid,
            } => ReadyCheckCompleted {
                party_index,
                party_guid,
            }
            .to_bytes(),
        };

        for recipient in &recipients {
            let _ = registry.send_current_packet(recipient.registration, bytes.clone());
        }
    }
}

pub(super) fn connected_group_member_txs_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
) -> Vec<PlayerRegistration> {
    registry
        .group_presences_in_order(&group.members)
        .into_iter()
        .map(|recipient| recipient.registration)
        .collect()
}

pub(super) fn send_group_packet_bytes_like_cpp(
    registry: &PlayerRegistry,
    bytes: Vec<u8>,
    recipients: &[PlayerRegistration],
) {
    for recipient in recipients {
        let _ = registry.send_current_packet(*recipient, bytes.clone());
    }
}

pub(super) fn send_party_uninvite_result_like_cpp(session: &WorldSession, result: u8) {
    session.send_packet_realm(&PartyCommandResult {
        name: String::new(),
        command: 1, // C++ PARTY_OP_UNINVITE
        result,
        result_data: 0,
        // C++ `WorldSession::SendPartyResult` always leaves `ResultGUID`
        // empty (`GroupHandler.cpp:53`).
        result_guid: ObjectGuid::EMPTY,
    });
}

/// Queue a realm-routed packet on the target's owning session.
///
/// Legacy C++ routes party-control packets through REALM
/// (`Opcodes.cpp:1826-1832`). This path receives only the target command
/// sender, so the target session performs the final realm-socket routing.
pub(super) async fn send_realm_packet_to_player_like_cpp(
    registry: &PlayerRegistry,
    registration: PlayerRegistration,
    recipient: ObjectGuid,
    packet_bytes: Vec<u8>,
) -> bool {
    let command = SessionCommand::SendRealmPacketLikeCpp(SendRealmPacketLikeCppCommand {
        recipient,
        packet_bytes,
    });
    match registry
        .send_current_command_timeout(registration, command, PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP)
        .await
    {
        Ok(()) => {
            // Test fixtures run a real command dispatcher on another task/thread.
            // Yielding here lets that receiver perform the same session-local
            // routing before packet assertions without adding a wrong-socket
            // production fallback.
            #[cfg(test)]
            tokio::task::yield_now().await;
            true
        }
        Err(
            error @ (crate::session::directory::PlayerDirectorySendError::Disconnected
            | crate::session::directory::PlayerDirectorySendError::StaleRegistration),
        ) => {
            warn!(recipient = %recipient, ?error, "realm-routed party command target stale or closed");
            false
        }
        Err(crate::session::directory::PlayerDirectorySendError::Full) => {
            warn!(recipient = %recipient, "timed out queueing realm-routed party packet");
            false
        }
    }
}

/// Queue the actual invite dialog without treating bounded-channel pressure as
/// an offline player.
///
/// C++ `HandlePartyInviteOpcode` stores the `GroupInvite` and then calls
/// `invitedPlayer->SendDirectMessage`; it does not turn a busy socket queue into
/// `ERR_BAD_PLAYER_NAME_S`. Rust therefore waits until the owning session drains
/// capacity or disconnects. Other existing group notifications keep their
/// finite timeout in [`send_realm_packet_to_player_like_cpp`]; this stronger
/// delivery rule is intentionally scoped to the invite state transition.
pub(super) async fn send_realm_party_invite_to_player_like_cpp(
    registry: &PlayerRegistry,
    registration: PlayerRegistration,
    recipient: ObjectGuid,
    packet_bytes: Vec<u8>,
) -> bool {
    let command = SessionCommand::SendRealmPacketLikeCpp(SendRealmPacketLikeCppCommand {
        recipient,
        packet_bytes,
    });
    match registry.send_current_command(registration, command).await {
        Ok(()) => {
            #[cfg(test)]
            tokio::task::yield_now().await;
            true
        }
        Err(error) => {
            warn!(recipient = %recipient, ?error, "realm-routed party invite target stale or closed");
            false
        }
    }
}

pub(super) fn role_changed_inform_like_cpp(
    party_index: u8,
    from: ObjectGuid,
    changed_unit: ObjectGuid,
    old_role: u8,
    new_role: u8,
) -> Vec<u8> {
    RoleChangedInform {
        party_index,
        from,
        changed_unit,
        old_role,
        new_role,
    }
    .to_bytes()
}

pub(super) fn role_poll_inform_like_cpp(party_index: i8, from: ObjectGuid) -> Vec<u8> {
    RolePollInform { party_index, from }.to_bytes()
}

pub(super) fn raid_target_update_single_like_cpp(
    party_index: u8,
    symbol: u8,
    target: ObjectGuid,
    changed_by: ObjectGuid,
) -> Vec<u8> {
    SendRaidTargetUpdateSingle {
        party_index,
        target,
        changed_by,
        symbol,
    }
    .to_bytes()
}

pub(super) fn raid_target_update_all_like_cpp(group: &GroupInfo) -> Vec<u8> {
    SendRaidTargetUpdateAll {
        party_index: group.group_category_like_cpp(),
        target_icons: group.target_icon_list_like_cpp(),
    }
    .to_bytes()
}

pub(super) fn raid_markers_changed_like_cpp(group: &GroupInfo) -> Vec<u8> {
    RaidMarkersChanged {
        party_index: group.group_category_like_cpp(),
        active_markers: group.active_raid_markers_mask_like_cpp(),
        raid_markers: group
            .raid_marker_list_like_cpp()
            .into_iter()
            .map(|marker| RaidMarker {
                transport_guid: marker.transport_guid,
                map_id: marker.map_id,
                position: marker.position,
            })
            .collect(),
    }
    .to_bytes()
}

pub(super) async fn queue_visible_gameobjects_or_spellclicks_refresh_like_cpp(
    group: &GroupInfo,
    registry: &PlayerRegistry,
    local_guid: ObjectGuid,
) {
    let recipients: Vec<_> = registry
        .group_presences_in_order(&group.members)
        .into_iter()
        .filter(|member| member.guid != local_guid)
        .collect();

    for member in recipients {
        match registry
            .send_current_command_timeout(
                member.registration,
                crate::session::mailbox::SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp,
                PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP,
            )
            .await
        {
            Ok(()) => {}
            Err(error) => warn!(
                member = %member.guid,
                ?error,
                "failed to queue visible gameobject refresh command"
            ),
        }
    }
}

/// Application mapping from the database-neutral commands emitted by
/// `GroupRegistry` to the SQLx-free persistence vocabulary. The vector order
/// remains the order selected by the aggregate and no registry guard survives
/// into adapter execution.
pub(crate) fn group_persistence_command_like_cpp(
    intent: GroupPersistenceIntentLikeCpp,
) -> RepresentedGroupPersistenceCommandLikeCpp {
    match intent {
        GroupPersistenceIntentLikeCpp::InsertGroup {
            db_store_id,
            leader_guid,
            loot_method,
            looter_guid,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid,
        } => RepresentedGroupPersistenceCommandLikeCpp::InsertGroup {
            db_store_id,
            leader_guid: leader_guid.counter() as u64,
            loot_method,
            looter_guid: looter_guid.counter() as u64,
            loot_threshold,
            group_flags,
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
            master_looter_guid: master_looter_guid.counter() as u64,
        },
        GroupPersistenceIntentLikeCpp::InsertMember {
            db_store_id,
            member_guid,
            member_flags,
            subgroup,
            roles,
        } => RepresentedGroupPersistenceCommandLikeCpp::InsertMember {
            db_store_id,
            member_guid: member_guid.counter() as u64,
            member_flags,
            subgroup,
            roles,
        },
        GroupPersistenceIntentLikeCpp::DeleteGroup { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteGroup { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteAllMembers { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteAllMembers { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteLfgData { db_store_id } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteLfgData { db_store_id }
        }
        GroupPersistenceIntentLikeCpp::DeleteMember { member_guid } => {
            RepresentedGroupPersistenceCommandLikeCpp::DeleteMember {
                member_guid: member_guid.counter() as u64,
            }
        }
        GroupPersistenceIntentLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateLeader {
            db_store_id,
            leader_guid: leader_guid.counter() as u64,
        },
        GroupPersistenceIntentLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateGroupType {
            db_store_id,
            group_flags,
        },
        GroupPersistenceIntentLikeCpp::UpdateMemberSubgroup {
            member_guid,
            subgroup,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberSubgroup {
            member_guid: member_guid.counter() as u64,
            subgroup,
        },
        GroupPersistenceIntentLikeCpp::UpdateMemberFlags { member_guid, flags } => {
            RepresentedGroupPersistenceCommandLikeCpp::UpdateMemberFlags {
                member_guid: member_guid.counter() as u64,
                flags,
            }
        }
        GroupPersistenceIntentLikeCpp::UpdateDifficulty {
            db_store_id,
            kind,
            difficulty_id,
        } => RepresentedGroupPersistenceCommandLikeCpp::UpdateDifficulty {
            db_store_id,
            kind: match kind {
                wow_social::group::GroupDifficultyKindLikeCpp::Dungeon => {
                    RepresentedGroupDifficultyKindLikeCpp::Dungeon
                }
                wow_social::group::GroupDifficultyKindLikeCpp::Raid => {
                    RepresentedGroupDifficultyKindLikeCpp::Raid
                }
                wow_social::group::GroupDifficultyKindLikeCpp::LegacyRaid => {
                    RepresentedGroupDifficultyKindLikeCpp::LegacyRaid
                }
            },
            difficulty_id,
        },
    }
}
