//! Chat handlers regression scenarios, part 2 of 3.
//!
//! Moved out of the chat.rs root under #654; every test is unchanged.

use super::*;

#[tokio::test]
async fn chat_message_rejects_client_universal_language_like_cpp() {
    let sender = ObjectGuid::create_player(1, 353);
    let nearby = ObjectGuid::create_player(1, 354);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet_with_language(
                ClientOpcodes::ChatMessageSay,
                LANG_UNIVERSAL_LIKE_CPP,
                "universal",
            ),
            ChatMsg::Say,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_message_rejects_unknown_language_like_cpp() {
    let sender = ObjectGuid::create_player(1, 370);
    let nearby = ObjectGuid::create_player(1, 371);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet_with_language(
                ClientOpcodes::ChatMessageSay,
                999,
                "unknown language",
            ),
            ChatMsg::Say,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn dead_player_cannot_send_say_like_cpp() {
    let sender = ObjectGuid::create_player(1, 357);
    let nearby = ObjectGuid::create_player(1, 358);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.set_player_alive_like_cpp(false);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "dead say"),
            ChatMsg::Say,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn low_level_player_cannot_send_say_or_yell_like_cpp() {
    let sender = ObjectGuid::create_player(1, 375);
    let nearby = ObjectGuid::create_player(1, 376);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.set_player_level_like_cpp(0);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "too low say"),
            ChatMsg::Say,
        )
        .await;
    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageYell, "too low yell"),
            ChatMsg::Yell,
        )
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("say level notification")),
        "You cannot say, yell or emote until you become level 1."
    );
    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("yell level notification")),
        "You cannot say, yell or emote until you become level 1."
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn configured_chat_level_requirements_gate_say_yell_and_emote_like_cpp() {
    let sender = ObjectGuid::create_player(1, 385);
    let nearby = ObjectGuid::create_player(1, 386);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.set_player_level_like_cpp(1);
    session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
        say: 2,
        yell: 2,
        emote: 2,
        ..ChatLevelRequirementsLikeCpp::default()
    });

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "too low say"),
            ChatMsg::Say,
        )
        .await;
    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageYell, "too low yell"),
            ChatMsg::Yell,
        )
        .await;
    session
        .handle_chat_emote(chat_emote_packet("too low emote"))
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("say level notification")),
        "You cannot say, yell or emote until you become level 2."
    );
    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("yell level notification")),
        "You cannot say, yell or emote until you become level 2."
    );
    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("emote level notification")),
        "You cannot say, yell or emote until you become level 2."
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn say_collapses_multiple_spaces_when_fake_message_preventing_enabled_like_cpp() {
    let sender = ObjectGuid::create_player(1, 383);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    session.set_chat_fake_message_preventing_like_cpp(true);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "hello   fake    spacing"),
            ChatMsg::Say,
        )
        .await;

    let echo = sender_rx.try_recv().expect("sender echo");
    assert_eq!(chat_slash_cmd(&echo), ChatMsg::Say as u8);
    assert_eq!(chat_text(&echo), "hello fake spacing");
}

#[tokio::test]
async fn say_preserves_multiple_spaces_when_fake_message_preventing_disabled_like_cpp() {
    let sender = ObjectGuid::create_player(1, 384);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "hello   fake    spacing"),
            ChatMsg::Say,
        )
        .await;

    let echo = sender_rx.try_recv().expect("sender echo");
    assert_eq!(chat_slash_cmd(&echo), ChatMsg::Say as u8);
    assert_eq!(chat_text(&echo), "hello   fake    spacing");
}

#[tokio::test]
async fn dead_player_party_chat_is_not_rejected_by_say_alive_gate_like_cpp() {
    let leader = ObjectGuid::create_player(1, 359);
    let member = ObjectGuid::create_player(1, 360);
    let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
    let (member_tx, member_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_player_alive_like_cpp(false);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageParty, "dead party"),
            ChatMsg::Party,
        )
        .await;

    assert_eq!(
        chat_slash_cmd(&leader_rx.try_recv().expect("leader party echo")),
        ChatMsg::PartyLeader as u8
    );
    assert_eq!(
        chat_slash_cmd(&member_rx.try_recv().expect("member party chat")),
        ChatMsg::PartyLeader as u8
    );
}

#[tokio::test]
async fn dead_player_cannot_send_chat_emote_like_cpp() {
    let sender = ObjectGuid::create_player(1, 368);
    let nearby = ObjectGuid::create_player(1, 369);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.set_player_alive_like_cpp(false);

    session
        .handle_chat_emote(chat_emote_packet("dead emote"))
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn low_level_player_cannot_send_chat_emote_like_cpp() {
    let sender = ObjectGuid::create_player(1, 377);
    let nearby = ObjectGuid::create_player(1, 378);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.set_player_level_like_cpp(0);

    session
        .handle_chat_emote(chat_emote_packet("too low emote"))
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("emote level notification")),
        "You cannot say, yell or emote until you become level 1."
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn whisper_rejects_client_universal_language_like_cpp() {
    let sender = ObjectGuid::create_player(1, 355);
    let target = ObjectGuid::create_player(1, 356);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());

    session
        .handle_chat_whisper(chat_whisper_packet_with_language(
            "Target",
            LANG_UNIVERSAL_LIKE_CPP,
            "universal",
        ))
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}

#[tokio::test]
async fn whisper_rejects_unknown_language_like_cpp() {
    let sender = ObjectGuid::create_player(1, 372);
    let target = ObjectGuid::create_player(1, 373);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());

    session
        .handle_chat_whisper(chat_whisper_packet_with_language(
            "Target",
            999,
            "unknown language",
        ))
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}

#[tokio::test]
async fn whisper_to_offline_player_sends_notfound_like_cpp() {
    let sender = ObjectGuid::create_player(1, 374);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_whisper(chat_whisper_packet("Missing", "hello"))
        .await;

    let bytes = sender_rx.try_recv().expect("notfound packet");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
    );
    assert_eq!(packet.read_bits(9).expect("name len"), 7);
    assert_eq!(packet.read_string(7).expect("name"), "Missing");
    assert!(packet.is_empty());
}

#[tokio::test]
async fn configured_whisper_level_requirement_blocks_non_gm_sender_like_cpp() {
    let sender = ObjectGuid::create_player(1, 387);
    let target = ObjectGuid::create_player(1, 388);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    session.set_player_level_like_cpp(1);
    session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
        whisper: 2,
        ..ChatLevelRequirementsLikeCpp::default()
    });

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "too low whisper"))
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("whisper level notification")),
        "You cannot whisper until you become level 2."
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}

#[tokio::test]
async fn configured_whisper_level_requirement_allows_gm_sender_like_cpp() {
    let sender = ObjectGuid::create_player(1, 389);
    let target = ObjectGuid::create_player(1, 390);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    session.set_player_level_like_cpp(1);
    session.set_player_game_master_like_cpp(true);
    session.set_chat_level_requirements_like_cpp(ChatLevelRequirementsLikeCpp {
        whisper: 2,
        ..ChatLevelRequirementsLikeCpp::default()
    });

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "gm whisper"))
        .await;

    assert_eq!(
        chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
        ChatMsg::Whisper as u8
    );
    assert_eq!(
        chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
        ChatMsg::WhisperInform as u8
    );
}

#[tokio::test]
async fn gm_silence_aura_rejects_non_whisper_chat_like_cpp() {
    let sender = ObjectGuid::create_player(1, 361);
    let nearby = ObjectGuid::create_player(1, 362);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.visible_auras.insert(1, gm_silence_aura(1));

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "muted"),
            ChatMsg::Say,
        )
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("silence notification")),
        "Silence is ON for Player361"
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn account_mute_time_rejects_chat_with_wait_notification_like_cpp() {
    let sender = ObjectGuid::create_player(1, 431);
    let nearby = ObjectGuid::create_player(1, 432);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    mute_session_for_seconds_like_cpp(&mut session, 3_600);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "mutetime"),
            ChatMsg::Say,
        )
        .await;

    let text = print_notification_text(&sender_rx.try_recv().expect("mute notification"));
    assert!(text.starts_with("You must wait "));
    assert!(text.ends_with(" before speaking again."));
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn account_mute_time_rejects_afk_without_changing_state_like_cpp() {
    let sender = ObjectGuid::create_player(1, 433);
    let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
    mute_session_for_seconds_like_cpp(&mut session, 3_600);

    session.handle_chat_afk(chat_away_packet("away")).await;

    assert_eq!(session.auto_reply_msg_like_cpp().as_deref(), Some(""));
    let text = print_notification_text(&sender_rx.try_recv().expect("mute notification"));
    assert!(text.starts_with("You must wait "));
    assert!(text.ends_with(" before speaking again."));
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn dedicated_addon_whisper_ignores_account_mute_time_like_cpp() {
    let sender = ObjectGuid::create_player(1, 434);
    let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
    mute_session_for_seconds_like_cpp(&mut session, 3_600);

    session
        .handle_chat_addon_message_whisper(chat_addon_whisper_packet("Missing", "ABC", "payload"))
        .await;

    let bytes = sender_rx
        .try_recv()
        .expect("dedicated addon whisper still sends missing target notice");
    let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::ChatPlayerNotfound as u16
    );
    assert_eq!(packet.read_bits(9).expect("name len"), 7);
    assert_eq!(packet.read_string(7).expect("name"), "Missing");
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_flood_regular_mutes_after_limit_for_next_message_like_cpp() {
    let sender = ObjectGuid::create_player(1, 435);
    let nearby = ObjectGuid::create_player(1, 436);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    let chat_policy = ChatPolicyCatalogsLikeCpp {
        flood: ChatFloodConfigLikeCpp {
            message_count: 2,
            message_delay_secs: 60,
            addon_message_count: 100,
            addon_message_delay_secs: 1,
            mute_time_secs: 10,
        },
        ..ChatPolicyCatalogsLikeCpp::default()
    };

    session
        .handle_chat_message_with_policy_like_cpp(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "first"),
            ChatMsg::Say,
            &chat_policy,
        )
        .await;
    session
        .handle_chat_message_with_policy_like_cpp(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "second"),
            ChatMsg::Say,
            &chat_policy,
        )
        .await;
    session
        .handle_chat_message_with_policy_like_cpp(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "third"),
            ChatMsg::Say,
            &chat_policy,
        )
        .await;

    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("first sender echo")),
        "first"
    );
    assert_eq!(
        chat_text(&nearby_rx.try_recv().expect("first nearby")),
        "first"
    );
    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("second sender echo")),
        "second"
    );
    assert_eq!(
        chat_text(&nearby_rx.try_recv().expect("second nearby")),
        "second"
    );
    let text = print_notification_text(&sender_rx.try_recv().expect("third mute notice"));
    assert!(text.starts_with("You must wait "));
    assert!(text.ends_with(" before speaking again."));
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_flood_addon_mutes_after_limit_for_next_generic_addon_like_cpp() {
    let leader = ObjectGuid::create_player(1, 437);
    let member = ObjectGuid::create_player(1, 438);
    let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
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
    session.set_chat_flood_config_like_cpp(ChatFloodConfigLikeCpp {
        message_count: 10,
        message_delay_secs: 1,
        addon_message_count: 2,
        addon_message_delay_secs: 60,
        mute_time_secs: 10,
    });

    session
        .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "first"))
        .await;
    session
        .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "second"))
        .await;
    session
        .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "third"))
        .await;

    assert_eq!(
        chat_text(&expect_addon_command(&member_command_rx).packet_bytes),
        "first"
    );
    assert_eq!(
        chat_text(&expect_addon_command(&member_command_rx).packet_bytes),
        "second"
    );
    assert!(member_command_rx.try_recv().is_err());
    assert!(leader_rx.try_recv().is_err());
}

#[test]
fn secs_to_full_time_string_matches_cpp_full_text_shape() {
    assert_eq!(secs_to_full_time_string_like_cpp(0), "0 Second.");
    assert_eq!(secs_to_full_time_string_like_cpp(1), "1 Second.");
    assert_eq!(secs_to_full_time_string_like_cpp(65), "1 Minute 5 Seconds.");
    assert_eq!(
        secs_to_full_time_string_like_cpp(90_061),
        "1 Day 1 Hour 1 Minute 1 Second."
    );
}

#[tokio::test]
async fn gm_silence_aura_rejects_afk_toggle_like_cpp() {
    let sender = ObjectGuid::create_player(1, 363);
    let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
    session.visible_auras.insert(1, gm_silence_aura(1));

    session.handle_chat_afk(chat_away_packet("away")).await;

    assert_eq!(session.auto_reply_msg_like_cpp().as_deref(), Some(""));
    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("silence notification")),
        "Silence is ON for Player363"
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn gm_silence_aura_rejects_dnd_toggle_like_cpp() {
    let sender = ObjectGuid::create_player(1, 430);
    let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
    session.visible_auras.insert(1, gm_silence_aura(1));

    session.handle_chat_dnd(chat_away_packet("busy")).await;

    assert_eq!(session.auto_reply_msg_like_cpp().as_deref(), Some(""));
    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("silence notification")),
        "Silence is ON for Player430"
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn gm_silence_aura_rejects_chat_emote_with_notification_like_cpp() {
    let sender = ObjectGuid::create_player(1, 431);
    let nearby = ObjectGuid::create_player(1, 432);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    session.visible_auras.insert(1, gm_silence_aura(1));

    session
        .handle_chat_emote(chat_emote_packet("muted emote"))
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("silence notification")),
        "Silence is ON for Player431"
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn gm_silence_aura_rejects_whisper_to_non_gm_like_cpp() {
    let sender = ObjectGuid::create_player(1, 364);
    let target = ObjectGuid::create_player(1, 365);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    bind_canonical_player_like_cpp(&player_registry, target, |_| {});
    session.visible_auras.insert(1, gm_silence_aura(1));

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "muted"))
        .await;

    assert_eq!(
        print_notification_text(&sender_rx.try_recv().expect("silence notification")),
        "Silence is ON for Player364"
    );
    assert!(sender_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}

#[tokio::test]
async fn gm_silence_aura_allows_whisper_to_gm_like_cpp() {
    let sender = ObjectGuid::create_player(1, 366);
    let target = ObjectGuid::create_player(1, 367);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    bind_canonical_player_like_cpp(&player_registry, target, |player| {
        player.set_game_master_like_cpp(true);
    });
    session.visible_auras.insert(1, gm_silence_aura(1));

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "gm only"))
        .await;

    assert_eq!(
        chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
        ChatMsg::Whisper as u8
    );
    assert_eq!(
        chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
        ChatMsg::WhisperInform as u8
    );
}

#[tokio::test]
async fn whisper_to_afk_player_sends_auto_reply_to_sender_like_cpp() {
    let sender = ObjectGuid::create_player(1, 379);
    let target = ObjectGuid::create_player(1, 380);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    bind_social_presence_like_cpp(
        &player_registry,
        target,
        crate::session::PLAYER_FLAGS_AFK_LIKE_CPP,
        "back soon",
    );

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "hello"))
        .await;

    assert_eq!(
        chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
        ChatMsg::Whisper as u8
    );
    assert_eq!(
        chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
        ChatMsg::WhisperInform as u8
    );
    let system = sender_rx.try_recv().expect("sender afk auto reply");
    assert_eq!(chat_slash_cmd(&system), ChatMsg::System as u8);
    assert_eq!(
        chat_text(&system),
        "Target is Away from Keyboard: back soon"
    );
}

#[tokio::test]
async fn whisper_to_dnd_player_sends_auto_reply_to_sender_like_cpp() {
    let sender = ObjectGuid::create_player(1, 381);
    let target = ObjectGuid::create_player(1, 382);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.identity.player_name = "Target".to_string();
    player_registry.register_or_replace(target, target_info, Default::default());
    bind_social_presence_like_cpp(
        &player_registry,
        target,
        crate::session::PLAYER_FLAGS_DND_LIKE_CPP,
        "busy",
    );

    session
        .handle_chat_whisper(chat_whisper_packet("Target", "hello"))
        .await;

    assert_eq!(
        chat_slash_cmd(&target_rx.try_recv().expect("target whisper")),
        ChatMsg::Whisper as u8
    );
    assert_eq!(
        chat_slash_cmd(&sender_rx.try_recv().expect("sender inform")),
        ChatMsg::WhisperInform as u8
    );
    let system = sender_rx.try_recv().expect("sender dnd auto reply");
    assert_eq!(chat_slash_cmd(&system), ChatMsg::System as u8);
    assert_eq!(
        chat_text(&system),
        "Target wishes to not be disturbed and cannot receive whisper messages: busy"
    );
}

#[tokio::test]
async fn party_addon_routes_to_same_subgroup_except_sender_like_cpp() {
    let leader = ObjectGuid::create_player(1, 401);
    let same_subgroup = ObjectGuid::create_player(1, 402);
    let other_subgroup = ObjectGuid::create_player(1, 403);
    let (mut session, player_registry, _leader_rx) = session_for_chat_routing_like_cpp(leader);
    let (same_tx, _same_rx) = flume::bounded(8);
    let (other_tx, _other_rx) = flume::bounded(8);
    let (same_command_tx, same_command_rx) = flume::bounded(8);
    let (other_command_tx, other_command_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        same_subgroup,
        broadcast_info_with_command_tx(same_subgroup, same_tx, same_command_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        other_subgroup,
        broadcast_info_with_command_tx(other_subgroup, other_tx, other_command_tx),
        Default::default(),
    );

    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.add_member(same_subgroup);
    group.add_member(other_subgroup);
    assert!(group.change_member_group_like_cpp(other_subgroup, 1));
    let group_guid = group.group_guid;
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_chat_addon_message(chat_addon_packet(ChatMsg::Party, "ABC", "payload"))
        .await;

    let command = expect_addon_command(&same_command_rx);
    assert_eq!(command.prefix, "ABC");
    assert_eq!(chat_slash_cmd(&command.packet_bytes), ChatMsg::Party as u8);
    assert_eq!(chat_language(&command.packet_bytes), LANG_ADDON_LIKE_CPP);
    assert!(other_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn addon_channel_config_blocks_addon_delivery_like_cpp() {
    let leader = ObjectGuid::create_player(1, 411);
    let member = ObjectGuid::create_player(1, 412);
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
    let chat_policy = ChatPolicyCatalogsLikeCpp {
        addon_channel: false,
        ..ChatPolicyCatalogsLikeCpp::default()
    };

    session
        .handle_chat_addon_message_with_policy_like_cpp(
            chat_addon_packet(ChatMsg::Party, "ABC", "payload"),
            &chat_policy,
        )
        .await;

    assert!(member_command_rx.try_recv().is_err());
}
