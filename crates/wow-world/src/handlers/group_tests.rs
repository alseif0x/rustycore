//! Behaviour tests for [`super`].
//!
//! Extracted from `group.rs`, which was 8,665 lines of which
//! 5,275 — 61% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::{
    PARTY_REALM_COMMAND_TIMEOUT_LIKE_CPP, current_group_guid_like_cpp,
    first_connected_group_member_like_cpp, group_persistence_command_like_cpp,
    party_player_info_like_cpp, send_group_new_leader_like_cpp, send_party_update,
    send_ready_check_events_like_cpp, sender_can_start_ready_check_like_cpp,
};
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::mailbox::{SendRealmPacketLikeCppCommand, SessionCommand};
use flume::bounded;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use wow_constants::{ClientOpcodes, ServerOpcodes};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::{ServerPacket, WorldPacket, packets::party::party_result};
use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, RepresentedGroupPersistenceOutcomeLikeCpp,
    RepresentedGroupPersistencePortLikeCpp, RepresentedGroupPersistenceRequestLikeCpp,
    SocialAddCandidateLoadOutcomeLikeCpp, SocialContactListLoadOutcomeLikeCpp,
    SocialPartyInviteLookupOutcomeLikeCpp, SocialPersistencePortLikeCpp,
    SocialRelationshipKindLikeCpp, SocialRelationshipStateLikeCpp,
};
use wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP;
use wow_social::group::{
    GroupInfo, GroupMemberCharacterLikeCpp, GroupRegistry, PendingInviteLikeCpp, PendingInvites,
    ReadyCheckEventLikeCpp,
};

use crate::session::{GroupInvitePolicyLikeCpp, WorldSession};

struct RecordingGroupPersistencePortLikeCpp {
    outcome: RepresentedGroupPersistenceOutcomeLikeCpp,
    requests: Mutex<Vec<RepresentedGroupPersistenceRequestLikeCpp>>,
}

impl RecordingGroupPersistencePortLikeCpp {
    fn new(outcome: RepresentedGroupPersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            outcome,
            requests: Mutex::new(Vec::new()),
        })
    }
}

impl RepresentedGroupPersistencePortLikeCpp for RecordingGroupPersistencePortLikeCpp {
    fn persist_group_commands_like_cpp(
        &self,
        request: RepresentedGroupPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, RepresentedGroupPersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

struct PartyInviteSocialPortLikeCpp {
    ignore: SocialPartyInviteLookupOutcomeLikeCpp,
    friend: SocialPartyInviteLookupOutcomeLikeCpp,
    calls: Mutex<Vec<String>>,
}

impl PartyInviteSocialPortLikeCpp {
    fn new(
        ignore: SocialPartyInviteLookupOutcomeLikeCpp,
        friend: SocialPartyInviteLookupOutcomeLikeCpp,
    ) -> Arc<Self> {
        Arc::new(Self {
            ignore,
            friend,
            calls: Mutex::new(Vec::new()),
        })
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl SocialPersistencePortLikeCpp for PartyInviteSocialPortLikeCpp {
    fn load_contacts_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _flags: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialContactListLoadOutcomeLikeCpp> {
        Box::pin(async { SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()) })
    }

    fn load_add_candidate_like_cpp<'a>(
        &'a self,
        _normalized_name: String,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialAddCandidateLoadOutcomeLikeCpp> {
        Box::pin(async { SocialAddCandidateLoadOutcomeLikeCpp::NotFound })
    }

    fn load_relationship_state_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SocialRelationshipStateLikeCpp> {
        Box::pin(async {
            SocialRelationshipStateLikeCpp {
                already_present: false,
                relationship_count: 0,
            }
        })
    }

    fn party_invite_target_ignores_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
        inviter_account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        self.calls.lock().unwrap().push(format!(
            "ignore:{target_guid}:{inviter_guid}:{inviter_account_id}"
        ));
        let outcome = self.ignore.clone();
        Box::pin(async move { outcome })
    }

    fn party_invite_target_has_friend_like_cpp<'a>(
        &'a self,
        target_guid: i64,
        inviter_guid: i64,
    ) -> PersistenceFutureLikeCpp<'a, SocialPartyInviteLookupOutcomeLikeCpp> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("friend:{target_guid}:{inviter_guid}"));
        let outcome = self.friend.clone();
        Box::pin(async move { outcome })
    }

    fn add_relationship_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn remove_relationship_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _kind: SocialRelationshipKindLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }

    fn set_contact_note_like_cpp<'a>(
        &'a self,
        _player_guid: i64,
        _target_guid: i64,
        _note: String,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        Box::pin(async { PersistenceOutcomeLikeCpp::Applied { rows: 0 } })
    }
}

fn test_session_command_dispatcher(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> (
    flume::Sender<SessionCommand>,
    flume::Receiver<SessionCommand>,
) {
    let (command_tx, command_rx) = flume::bounded(0);
    let (observed_tx, observed_rx) = flume::unbounded();
    std::thread::spawn(move || {
        let mut party_sequences = std::collections::HashMap::<u8, i32>::new();
        while let Ok(command) = command_rx.recv() {
            match command {
                SessionCommand::SendRealmPacketLikeCpp(command) if command.recipient == guid => {
                    let _ = send_tx.send(command.packet_bytes);
                }
                SessionCommand::SendPartyUpdateLikeCpp(mut command)
                    if command.recipient == guid =>
                {
                    let sequence = party_sequences
                        .entry(command.party_update.party_index)
                        .or_default();
                    *sequence += 1;
                    command.party_update.sequence_num = *sequence;
                    let _ = send_tx.send(command.party_update.to_bytes());
                    for packet in command.member_full_state_packets {
                        let _ = send_tx.send(packet);
                    }
                }
                command => {
                    let _ = observed_tx.send(command);
                }
            }
        }
    });
    (command_tx, observed_rx)
}

fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> PlayerSessionRegistrationLikeCpp {
    let (command_tx, _observed_rx) = test_session_command_dispatcher(guid, send_tx.clone());
    broadcast_info_with_command_tx(guid, send_tx, command_tx)
}

fn recv_dispatched_packet(rx: &flume::Receiver<Vec<u8>>, label: &str) -> Vec<u8> {
    rx.recv_timeout(std::time::Duration::from_secs(1))
        .unwrap_or_else(|error| panic!("{label}: {error}"))
}

fn broadcast_info_with_command_tx(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<SessionCommand>,
) -> PlayerSessionRegistrationLikeCpp {
    PlayerSessionRegistrationLikeCpp {
        identity: PlayerDirectoryIdentityLikeCpp {
            player_name: format!("Player{}", guid.low_value()),
            account_id: 1,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

fn bind_canonical_party_players_like_cpp(
    registry: &PlayerRegistry,
    players: impl IntoIterator<Item = ObjectGuid>,
) -> crate::session::SharedCanonicalMapManager {
    let canonical = registry
        .fixture_canonical_map_manager_like_cpp()
        .expect("canonical player fixture manager");
    let mut manager = canonical.lock().unwrap();
    let map = manager.create_world_map(0, 0).map_mut();
    for guid in players {
        if map.get_typed_player(guid).is_some() {
            continue;
        }
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(0, 0).unwrap();
        player.unit_mut().world_mut().relocate(Position::ZERO);
        player.unit_mut().world_mut().object_mut().add_to_world();
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(100);
        map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }
    drop(manager);
    canonical
}

fn packed_guid_bytes(guid: ObjectGuid) -> Vec<u8> {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&guid);
    pkt.into_data()
}

fn set_loot_method_packet(
    has_party_index: bool,
    method: u8,
    master: ObjectGuid,
    threshold: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(has_party_index);
    pkt.write_uint8(method);
    pkt.write_packed_guid(&master);
    pkt.write_uint32(threshold);
    if has_party_index {
        pkt.write_uint8(0);
    }
    pkt.reset_read();
    pkt
}

fn opt_out_of_loot_packet(pass_on_loot: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(pass_on_loot);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn convert_raid_packet(raid: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(raid);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn change_sub_group_packet(
    target_guid: ObjectGuid,
    new_subgroup: u8,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_packed_guid(&target_guid);
    pkt.write_uint8(new_subgroup);
    pkt.write_bit(party_index.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn swap_sub_groups_packet(
    first_target: ObjectGuid,
    second_target: ObjectGuid,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_packed_guid(&first_target);
    pkt.write_packed_guid(&second_target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn set_assistant_leader_packet(
    target: ObjectGuid,
    apply: bool,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bit(apply);
    pkt.write_packed_guid(&target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn set_party_leader_packet(target: ObjectGuid, party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_packed_guid(&target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn party_uninvite_packet(target: ObjectGuid, party_index: Option<u8>, reason: &str) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bits(reason.len() as u32, 8);
    pkt.write_packed_guid(&target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    pkt.write_string(reason);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn party_invite_packet(
    target_guid: ObjectGuid,
    target_name: &str,
    party_index: Option<u8>,
    proposed_roles: u32,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.flush_bits();
    pkt.write_bits(target_name.len() as u32, 9);
    pkt.write_bits(0, 9);
    pkt.write_uint32(proposed_roles);
    pkt.write_packed_guid(&target_guid);
    pkt.write_string(target_name);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    pkt.reset_read();
    pkt
}

fn party_invite_response_packet(
    accept: bool,
    party_index: Option<u8>,
    roles: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bit(accept);
    pkt.write_bit(roles.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    }
    if let Some(roles) = roles {
        pkt.write_uint8(roles);
    }
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn party_command_result_code(bytes: &[u8]) -> u8 {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyCommandResult as u16
    );
    let _ = packet.read_bits(9).expect("name len");
    let _ = packet.read_bits(4).expect("party operation");
    packet.read_bits(6).expect("party result") as u8
}

fn party_invite_can_accept(bytes: &[u8]) -> bool {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyInvite as u16
    );
    packet.read_bit().expect("can accept")
}

fn leave_group_packet(party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn set_everyone_is_assistant_packet(
    everyone_is_assistant: bool,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bit(everyone_is_assistant);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn silence_party_talker_packet(target: ObjectGuid, silent: bool) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bytes(&target.to_raw_bytes());
    pkt.write_bit(silent);
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn set_party_assignment_packet(
    assignment: u8,
    target: ObjectGuid,
    apply: bool,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_bit(apply);
    pkt.write_uint8(assignment);
    pkt.write_packed_guid(&target);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn set_role_packet(target: ObjectGuid, role: u8, party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_packed_guid(&target);
    pkt.write_uint8(role);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn update_raid_target_packet(
    target: ObjectGuid,
    symbol: i8,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_packed_guid(&target);
    pkt.write_int8(symbol);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn request_party_join_updates_packet(party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn assert_raid_markers_packet_like_cpp(
    bytes: &[u8],
    expected_active_markers: u32,
    expected_positions: &[Position],
) {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::RaidMarkersChanged as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), GROUP_CATEGORY_HOME_LIKE_CPP);
    assert_eq!(pkt.read_uint32().unwrap(), expected_active_markers);
    assert_eq!(
        pkt.read_bits(4).unwrap(),
        u32::try_from(expected_positions.len()).unwrap()
    );
    pkt.flush_bits();
    for expected_position in expected_positions {
        assert_eq!(pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(pkt.read_uint32().unwrap(), 571);
        assert_eq!(pkt.read_float().unwrap(), expected_position.x);
        assert_eq!(pkt.read_float().unwrap(), expected_position.y);
        assert_eq!(pkt.read_float().unwrap(), expected_position.z);
    }
    assert!(pkt.is_empty());
}

fn clear_raid_marker_packet(marker_id: u8) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint8(marker_id);
    pkt.reset_read();
    pkt
}

fn initiate_role_poll_packet(party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn request_party_member_stats_packet(
    target_guid: ObjectGuid,
    party_index: Option<u8>,
) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_packed_guid(&target_guid);
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn do_ready_check_packet(party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    if let Some(party_index) = party_index {
        pkt.write_uint8(party_index);
    } else {
        pkt.flush_bits();
    }
    pkt.reset_read();
    pkt
}

fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = bounded::<Vec<u8>>(4);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    (session, send_rx)
}

fn lfg_group_like_cpp(leader: ObjectGuid, member_count: usize) -> GroupInfo {
    let mut group = GroupInfo::new(leader);
    for counter in 100..(100 + member_count as i64 - 1) {
        assert!(group.add_member(ObjectGuid::create_player(1, counter)));
    }
    group.group_flags |= wow_social::group::GROUP_FLAG_LFG_LIKE_CPP;
    group.lfg_kicks_left_like_cpp = wow_social::group::LFG_GROUP_MAX_KICKS_LIKE_CPP;
    group
}

fn lfg_uninvite_session_like_cpp(
    group: GroupInfo,
    sender_guid: ObjectGuid,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    Arc<GroupRegistry>,
    u64,
) {
    let (mut session, send_rx) = make_session_with_send();
    let group_registry = Arc::new(GroupRegistry::default());
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_player_guid(Some(sender_guid));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::new(
        PlayerRegistry::with_canonical_player_fixtures_like_cpp(),
    ));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    (session, send_rx, group_registry, group_guid)
}

fn low_level_raid_packet() -> WorldPacket {
    WorldPacket::new_empty()
}

fn minimap_ping_packet(x: f32, y: f32, party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_float(x);
    pkt.write_float(y);
    if let Some(idx) = party_index {
        pkt.write_uint8(idx);
    }
    pkt.flush_bits();
    pkt.reset_read();
    pkt
}

fn random_roll_packet(min: i32, max: i32, party_index: Option<u8>) -> WorldPacket {
    let mut pkt = WorldPacket::new_empty();
    pkt.write_bit(party_index.is_some());
    pkt.write_int32(min);
    pkt.write_int32(max);
    if let Some(idx) = party_index {
        pkt.write_uint8(idx);
    }
    pkt.reset_read();
    pkt
}

fn assert_random_roll_packet(
    bytes: &[u8],
    roller: ObjectGuid,
    account_id: u32,
    min: i32,
    max: i32,
) -> i32 {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(pkt.read_uint16().unwrap(), ServerOpcodes::RandomRoll as u16);
    assert_eq!(pkt.read_guid().unwrap(), roller);
    assert_eq!(
        pkt.read_guid().unwrap(),
        ObjectGuid::new((HighGuid::WowAccount as i64) << 58, i64::from(account_id))
    );
    assert_eq!(pkt.read_int32().unwrap(), min);
    assert_eq!(pkt.read_int32().unwrap(), max);
    let result = pkt.read_int32().unwrap();
    assert!((min..=max).contains(&result));
    assert_eq!(pkt.remaining(), 0);
    result
}

// ── canonical group lookup architectural tests ─────────────────────────────

#[path = "group_tests/combat.rs"]
mod combat;
#[path = "group_tests/group_1.rs"]
mod group_1;
#[path = "group_tests/group_2.rs"]
mod group_2;
#[path = "group_tests/group_3.rs"]
mod group_3;
#[path = "group_tests/group_4.rs"]
mod group_4;
#[path = "group_tests/group_5.rs"]
mod group_5;
#[path = "group_tests/instance.rs"]
mod instance;
#[path = "group_tests/login.rs"]
mod login;
#[path = "group_tests/loot.rs"]
mod loot;
#[path = "group_tests/misc.rs"]
mod misc;
#[path = "group_tests/movement.rs"]
mod movement;
#[path = "group_tests/quest.rs"]
mod quest;
#[path = "group_tests/spell.rs"]
mod spell;
