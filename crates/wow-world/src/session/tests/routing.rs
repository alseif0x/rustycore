// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[tokio::test]
async fn realm_only_party_commands_never_use_instance_after_connect_to_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_guid(Some(player_guid));
    session.state = SessionState::LoggedIn;

    let member_full_state = vec![0x59, 0x27, 0xAA];
    session
        .session_command_tx()
        .try_send(SessionCommand::SendPartyUpdateLikeCpp(
            SendPartyUpdateLikeCppCommand {
                recipient: player_guid,
                party_update: wow_packet::packets::party::PartyUpdate {
                    party_flags: 0,
                    party_index: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                    party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                    my_index: 0,
                    party_guid: group_guid,
                    sequence_num: 999,
                    leader_guid: player_guid,
                    leader_faction_group: 0,
                    player_list: Vec::new(),
                    loot_settings: None,
                    difficulty_settings: None,
                },
                member_full_state_packets: vec![member_full_state.clone()],
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    let update = realm_rx.try_recv().expect("realm PartyUpdate");
    assert_eq!(
        u16::from_le_bytes([update[0], update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(realm_rx.try_recv().unwrap(), member_full_state);
    assert!(instance_rx.try_recv().is_err());

    let invite = vec![0xBD, 0x25, 0xCC];
    session
        .session_command_tx()
        .try_send(SessionCommand::SendRealmPacketLikeCpp(
            SendRealmPacketLikeCppCommand {
                recipient: player_guid,
                packet_bytes: invite.clone(),
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(realm_rx.try_recv().unwrap(), invite);
    assert!(instance_rx.try_recv().is_err());

    let wrong_recipient = ObjectGuid::create_player(1, 43);
    session
        .session_command_tx()
        .try_send(SessionCommand::SendRealmPacketLikeCpp(
            SendRealmPacketLikeCppCommand {
                recipient: wrong_recipient,
                packet_bytes: invite.clone(),
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(realm_rx.try_recv().is_err());

    session
        .session_command_tx()
        .try_send(SessionCommand::SendPartyUpdateLikeCpp(
            SendPartyUpdateLikeCppCommand {
                recipient: wrong_recipient,
                party_update: wow_packet::packets::party::PartyUpdate {
                    party_flags: 0,
                    party_index: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                    party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                    my_index: 0,
                    party_guid: group_guid,
                    sequence_num: 0,
                    leader_guid: player_guid,
                    leader_faction_group: 0,
                    player_list: Vec::new(),
                    loot_settings: None,
                    difficulty_settings: None,
                },
                member_full_state_packets: Vec::new(),
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(realm_rx.try_recv().is_err());

    session.state = SessionState::Authed;
    session
        .session_command_tx()
        .try_send(SessionCommand::SendRealmPacketLikeCpp(
            SendRealmPacketLikeCppCommand {
                recipient: player_guid,
                packet_bytes: invite,
            },
        ))
        .unwrap();
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(realm_rx.try_recv().is_err());
    assert!(instance_rx.try_recv().is_err());
}

/// Parses the destroyed `PartyUpdate` that C++
/// `Group::SendUpdateDestroyGroupToPlayer` (`Group.cpp:917-926`) sends so
/// the removed member's client tears down its party frames.
pub(super) fn assert_destroyed_party_update_like_cpp(bytes: &[u8], group_guid: u64) {
    let mut packet = WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(
        packet.read_uint16().expect("party flags"),
        wow_social::group::GROUP_FLAG_DESTROYED_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party index"),
        wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party type"),
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(packet.read_int32().expect("my index"), -1);
    assert_eq!(
        packet.read_packed_guid().expect("party guid"),
        ObjectGuid::create_group(group_guid)
    );
    let _sequence_num = packet.read_int32().expect("sequence num");
    assert_eq!(
        packet.read_packed_guid().expect("leader guid"),
        ObjectGuid::EMPTY
    );
}

/// (4) Packet is NOT sent when `instance_id` in command does not match session instance.
/// Slice 4A.1b requirement — instance separation, no cross-instance delivery.
#[tokio::test]
async fn send_if_visible_command_rejected_on_wrong_instance_id_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let source_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1004);
    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    // session has no canonical map manager → instance_id fallback is 0
    session.client_visible_guids_like_cpp.insert(source_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendIfVisibleLikeCpp(
            SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id: 571,
                instance_id: 99, // different instance
                packet_bytes: vec![0x88],
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "must not send when instance_id does not match"
    );
}

#[tokio::test]
async fn party_uninvite_wire_dispatch_parses_packed_guid_and_honors_logged_in_gate_like_cpp() {
    fn packet(target: ObjectGuid) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint16(ClientOpcodes::PartyUninvite as u16);
        packet.write_bit(false);
        packet.write_bits(3, 8);
        packet.write_packed_guid(&target);
        packet.write_string("bye");
        packet.flush_bits();
        packet.reset_read();
        packet
    }

    let player = ObjectGuid::create_player(1, 101);
    let target = ObjectGuid::create_player(1, 202);
    let (mut logged_in, _pkt_tx, send_rx) = make_session();
    logged_in.set_state(SessionState::LoggedIn);
    logged_in.set_player_guid(Some(player));

    logged_in
        .dispatch_packet(&ObjectMgrCatalogsLikeCpp::default(), packet(target))
        .await;

    let result = send_rx.try_recv().expect("PartyCommandResult");
    assert_eq!(
        u16::from_le_bytes([result[0], result[1]]),
        ServerOpcodes::PartyCommandResult as u16
    );
    assert!(send_rx.try_recv().is_err());

    let (mut authed, _pkt_tx, send_rx) = make_session();
    authed.set_player_guid(Some(player));
    authed
        .dispatch_packet(&ObjectMgrCatalogsLikeCpp::default(), packet(target))
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "LoggedIn metadata must reject PartyUninvite while the session is Authed"
    );
}

#[tokio::test]
async fn move_set_vehicle_rec_id_ack_wire_dispatch_reaches_handler_only_when_logged_in() {
    fn packet(mover: ObjectGuid) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_uint16(ClientOpcodes::MoveSetVehicleRecIdAck as u16);
        wow_packet::packets::movement::MovementInfo {
            guid: mover,
            time: 12_345,
            position: Position::new(1.25, -2.5, 3.75, 0.5),
            ..Default::default()
        }
        .write(&mut packet);
        packet.write_int32(-77);
        packet.write_int32(9_001);
        packet.reset_read();
        packet
    }

    use crate::handlers::movement::take_move_set_vehicle_rec_id_ack_handler_calls_for_test;

    let mover = ObjectGuid::create_player(1, 101);
    assert_eq!(take_move_set_vehicle_rec_id_ack_handler_calls_for_test(), 0);

    let (mut logged_in, _pkt_tx, send_rx) = make_session();
    logged_in.set_state(SessionState::LoggedIn);
    logged_in.set_player_guid(Some(mover));
    logged_in
        .dispatch_packet(&ObjectMgrCatalogsLikeCpp::default(), packet(mover))
        .await;

    assert_eq!(take_move_set_vehicle_rec_id_ack_handler_calls_for_test(), 1);
    assert!(
        send_rx.try_recv().is_err(),
        "C++ HandleMoveSetVehicleRecAck sends no response"
    );

    let (mut authed, _pkt_tx, send_rx) = make_session();
    authed.set_player_guid(Some(mover));
    authed
        .dispatch_packet(&ObjectMgrCatalogsLikeCpp::default(), packet(mover))
        .await;

    assert_eq!(
        take_move_set_vehicle_rec_id_ack_handler_calls_for_test(),
        0,
        "LoggedIn metadata must reject the ACK while the session is Authed"
    );
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn dispatch_routes_send_text_emote_to_handler_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 101);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.player_name = Some("Emoter".to_string());
    session.set_emotes_text_store_like_cpp(Arc::new(wow_data::EmotesTextStore::from_entries([
        wow_data::EmotesTextEntry {
            id: 101,
            name: "wave".to_string(),
            emote_id: 3,
        },
    ])));

    let mut packet = WorldPacket::new_empty();
    packet.write_uint16(ClientOpcodes::SendTextEmote as u16);
    packet.write_packed_guid(&ObjectGuid::EMPTY);
    packet.write_int32(101);
    packet.write_int32(-1);
    packet.write_uint32(0);
    packet.write_int32(0);
    let bytes = packet.data().to_vec();

    session
        .dispatch_packet(
            &ObjectMgrCatalogsLikeCpp::default(),
            WorldPacket::from_bytes(&bytes),
        )
        .await;

    let mut anim = WorldPacket::from_bytes(&send_rx.try_recv().expect("anim emote"));
    assert_eq!(
        anim.read_uint16().expect("anim opcode"),
        ServerOpcodes::Emote as u16
    );
    assert_eq!(anim.read_packed_guid().expect("anim source"), player_guid);
    assert_eq!(anim.read_int32().expect("anim emote"), 3);

    let mut text = WorldPacket::from_bytes(&send_rx.try_recv().expect("text emote"));
    assert_eq!(
        text.read_uint16().expect("text opcode"),
        ServerOpcodes::TextEmote as u16
    );
    assert_eq!(text.read_packed_guid().expect("text source"), player_guid);
    let _account_guid = text.read_packed_guid().expect("source account");
    assert_eq!(text.read_int32().expect("text emote id"), 101);
    assert_eq!(text.read_int32().expect("text sound index"), -1);
    assert_eq!(
        text.read_packed_guid().expect("text target"),
        ObjectGuid::EMPTY
    );
    assert!(send_rx.try_recv().is_err());
}
