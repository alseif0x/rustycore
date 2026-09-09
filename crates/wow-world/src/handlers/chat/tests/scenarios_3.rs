//! Chat handlers regression scenarios, part 3 of 3.
//!
//! Moved out of the chat.rs root under #654; every test is unchanged.

use super::*;

#[tokio::test]
async fn addon_whisper_routes_to_named_registered_target_like_cpp() {
    let sender = ObjectGuid::create_player(1, 421);
    let target = ObjectGuid::create_player(1, 422);
    let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, _target_rx) = flume::bounded(8);
    let (target_command_tx, target_command_rx) = flume::bounded(8);
    let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());

    session
        .handle_chat_addon_message_whisper(chat_addon_whisper_packet("Target", "ABC", "payload"))
        .await;

    let command = expect_addon_command(&target_command_rx);
    assert_eq!(command.prefix, "ABC");
    assert_eq!(
        chat_slash_cmd(&command.packet_bytes),
        ChatMsg::Whisper as u8
    );
    assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
    assert_eq!(chat_text(&command.packet_bytes), "payload");
}

#[tokio::test]
async fn targeted_addon_whisper_routes_logged_payload_like_cpp() {
    let sender = ObjectGuid::create_player(1, 425);
    let target = ObjectGuid::create_player(1, 426);
    let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, _target_rx) = flume::bounded(8);
    let (target_command_tx, target_command_rx) = flume::bounded(8);
    let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());

    session
        .handle_chat_addon_message_targeted(chat_addon_targeted_packet(
            ChatMsg::Whisper,
            "ABC",
            "payload",
            "Target",
            ObjectGuid::EMPTY,
            true,
        ))
        .await;

    let command = expect_addon_command(&target_command_rx);
    assert_eq!(command.prefix, "ABC");
    assert_eq!(
        chat_slash_cmd(&command.packet_bytes),
        ChatMsg::Whisper as u8
    );
    assert_eq!(
        chat_language(&command.packet_bytes),
        LANG_ADDON_LOGGED_LIKE_CPP
    );
    assert_eq!(chat_text(&command.packet_bytes), "payload");
}

#[tokio::test]
async fn targeted_party_addon_uses_group_routing_like_cpp() {
    let leader = ObjectGuid::create_player(1, 427);
    let member = ObjectGuid::create_player(1, 428);
    let (mut session, player_registry, _leader_rx) = session_for_chat_routing_like_cpp(leader);
    let (member_tx, _member_rx) = flume::bounded(8);
    let (member_command_tx, member_command_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        member,
        broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        Default::default(),
    );

    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_chat_addon_message_targeted(chat_addon_targeted_packet(
            ChatMsg::Party,
            "ABC",
            "payload",
            "IgnoredTarget",
            ObjectGuid::EMPTY,
            false,
        ))
        .await;

    let command = expect_addon_command(&member_command_rx);
    assert_eq!(command.prefix, "ABC");
    assert_eq!(chat_slash_cmd(&command.packet_bytes), ChatMsg::Party as u8);
    assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
}

#[tokio::test]
async fn addon_whisper_missing_target_sends_notfound_like_cpp() {
    let sender = ObjectGuid::create_player(1, 431);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_addon_message_whisper(chat_addon_whisper_packet("Missing", "ABC", "payload"))
        .await;

    let notfound = sender_rx.try_recv().expect("notfound packet");
    let mut packet = wow_packet::WorldPacket::from_bytes(&notfound);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
    );
}

#[tokio::test]
async fn addon_whisper_level_requirement_blocks_delivery_like_cpp() {
    let sender = ObjectGuid::create_player(1, 441);
    let target = ObjectGuid::create_player(1, 442);
    let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, _target_rx) = flume::bounded(8);
    let (target_command_tx, target_command_rx) = flume::bounded(8);
    let mut target_info = broadcast_info_with_command_tx(target, target_tx, target_command_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    session.set_player_level_like_cpp(1);
    session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
        whisper: 2,
        ..ChatLevelRequirementsLikeCpp::default()
    });

    session
        .handle_chat_addon_message_whisper(chat_addon_whisper_packet("Target", "ABC", "payload"))
        .await;

    assert!(target_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn addon_command_delivers_only_when_prefix_registered_like_cpp() {
    let receiver = ObjectGuid::create_player(1, 501);
    let (mut session, _, send_rx) = session_for_chat_routing_like_cpp(receiver);
    session.set_state(crate::session::SessionState::LoggedIn);
    session.filter_addon_messages = true;
    session.registered_addon_prefixes = vec!["ABC".to_string()];
    let packet = ChatPkt {
        msg_type: ChatMsg::Raid,
        language: LANG_ADDON_LIKE_CPP,
        sender_guid: ObjectGuid::create_player(1, 502),
        sender_name: "Sender".to_string(),
        target_guid: ObjectGuid::EMPTY,
        target_name: String::new(),
        prefix: "ABC".to_string(),
        channel: String::new(),
        text: "payload".to_string(),
        virtual_realm: 0,
    };

    session
        .session_command_tx()
        .try_send(SessionCommand::SendAddonIfRegisteredLikeCpp(
            SendAddonIfRegisteredLikeCppCommand {
                prefix: "XYZ".to_string(),
                packet_bytes: packet.to_bytes(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(send_rx.try_recv().is_err());

    let packet = ChatPkt {
        prefix: "ABC".to_string(),
        ..packet
    };
    session
        .session_command_tx()
        .try_send(SessionCommand::SendAddonIfRegisteredLikeCpp(
            SendAddonIfRegisteredLikeCppCommand {
                prefix: "ABC".to_string(),
                packet_bytes: packet.to_bytes(),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;
    let delivered = send_rx.try_recv().expect("registered prefix delivered");
    assert_eq!(chat_slash_cmd(&delivered), ChatMsg::Raid as u8);
    assert_eq!(chat_language(&delivered), LANG_ADDON_LIKE_CPP);
}
