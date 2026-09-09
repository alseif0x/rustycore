//! Chat handlers regression scenarios, part 1 of 3.
//!
//! Moved out of the chat.rs root under #654; every test is unchanged.

use super::*;

#[tokio::test]
async fn chat_report_filtered_empty_stub_sends_no_response_like_cpp() {
    let sender = ObjectGuid::create_player(1, 100);
    let (mut session, _registry, send_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_report_filtered(wow_packet::WorldPacket::new_empty())
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_report_ignored_notifies_ignored_player_like_cpp() {
    let reporter = ObjectGuid::create_player(1, 101);
    let ignored = ObjectGuid::create_player(1, 102);
    let (mut session, registry, reporter_rx) = session_for_chat_routing_like_cpp(reporter);
    let (ignored_tx, ignored_rx) = flume::bounded(8);
    registry.register_or_replace(
        ignored,
        broadcast_info(ignored, ignored_tx),
        Default::default(),
    );

    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_packed_guid(&ignored);
    writer.write_uint8(0);
    session
        .handle_chat_report_ignored(wow_packet::WorldPacket::from_bytes(writer.data()))
        .await;

    assert_eq!(
        chat_slash_cmd(&ignored_rx.try_recv().expect("ignored notification")),
        ChatMsg::Ignored as u8
    );
    assert!(reporter_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_register_addon_prefixes_accumulates_and_updates_filter_like_cpp() {
    let sender = ObjectGuid::create_player(1, 103);
    let (mut session, _registry, send_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_register_addon_prefixes(chat_register_addon_prefixes_packet(&["ABC", "DEF"]))
        .await;
    assert_eq!(session.registered_addon_prefixes, vec!["ABC", "DEF"]);
    assert!(session.filter_addon_messages);

    let too_many = vec!["X"; ChatRegisterAddonPrefixes::MAX_PREFIXES - 1];
    session
        .handle_chat_register_addon_prefixes(chat_register_addon_prefixes_packet(&too_many))
        .await;
    assert_eq!(
        session.registered_addon_prefixes.len(),
        ChatRegisterAddonPrefixes::MAX_PREFIXES + 1
    );
    assert!(!session.filter_addon_messages);
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_chat_routes_only_to_sender_subgroup_like_cpp() {
    let leader = ObjectGuid::create_player(1, 101);
    let same_subgroup = ObjectGuid::create_player(1, 102);
    let other_subgroup = ObjectGuid::create_player(1, 103);
    let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
    let (same_tx, same_rx) = flume::bounded(8);
    let (other_tx, other_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        same_subgroup,
        broadcast_info(same_subgroup, same_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        other_subgroup,
        broadcast_info(other_subgroup, other_tx),
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
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageParty, "party"),
            ChatMsg::Party,
        )
        .await;

    assert_eq!(
        chat_slash_cmd(&leader_rx.try_recv().expect("leader party echo")),
        ChatMsg::PartyLeader as u8
    );
    assert_eq!(
        chat_slash_cmd(&same_rx.try_recv().expect("same subgroup party chat")),
        ChatMsg::PartyLeader as u8
    );
    assert!(other_rx.try_recv().is_err());
}

#[tokio::test]
async fn raid_chat_routes_to_all_raid_members_like_cpp() {
    let leader = ObjectGuid::create_player(1, 201);
    let member = ObjectGuid::create_player(1, 202);
    let (mut session, player_registry, leader_rx) = session_for_chat_routing_like_cpp(leader);
    let (member_tx, member_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.add_member(member);
    let group_guid = group.group_guid;
    let group_registry = Arc::new(wow_social::group::GroupRegistry::default());
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageRaid, "raid"),
            ChatMsg::Raid,
        )
        .await;

    assert_eq!(
        chat_slash_cmd(&leader_rx.try_recv().expect("leader raid echo")),
        ChatMsg::RaidLeader as u8
    );
    assert_eq!(
        chat_slash_cmd(&member_rx.try_recv().expect("member raid chat")),
        ChatMsg::RaidLeader as u8
    );
}

#[tokio::test]
async fn raid_warning_in_party_requires_party_raid_warnings_config_like_cpp() {
    let leader = ObjectGuid::create_player(1, 203);
    let member = ObjectGuid::create_player(1, 204);
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

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageRaidWarning, "blocked"),
            ChatMsg::RaidWarning,
        )
        .await;

    assert!(leader_rx.try_recv().is_err());
    assert!(member_rx.try_recv().is_err());
}

#[tokio::test]
async fn party_raid_warnings_config_allows_party_raid_warning_like_cpp() {
    let leader = ObjectGuid::create_player(1, 205);
    let member = ObjectGuid::create_player(1, 206);
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
    session.set_party_raid_warnings_like_cpp(true);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageRaidWarning, "party warning"),
            ChatMsg::RaidWarning,
        )
        .await;

    assert_eq!(
        chat_slash_cmd(&leader_rx.try_recv().expect("leader warning")),
        ChatMsg::RaidWarning as u8
    );
    assert_eq!(
        chat_slash_cmd(&member_rx.try_recv().expect("member warning")),
        ChatMsg::RaidWarning as u8
    );
}

#[tokio::test]
async fn guild_chat_does_not_leak_to_nearby_players_without_guild_registry_like_cpp() {
    let sender = ObjectGuid::create_player(1, 301);
    let nearby = ObjectGuid::create_player(1, 302);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageGuild, "guild"),
            ChatMsg::Guild,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
    assert!(!session.is_disconnecting());
}

#[tokio::test]
async fn channel_chat_does_not_leak_without_channel_mgr_like_cpp() {
    let sender = ObjectGuid::create_player(1, 321);
    let nearby = ObjectGuid::create_player(1, 322);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_channel_message(chat_channel_message_packet("General", "channel"))
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
    assert!(!session.is_disconnecting());
}

#[tokio::test]
async fn update_aadc_status_forces_chat_enabled_like_cpp() {
    let sender = ObjectGuid::create_player(1, 331);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_bit(true);
    writer.flush_bits();

    session
        .handle_update_aadc_status(wow_packet::WorldPacket::from_bytes(writer.data()))
        .await;

    let bytes = sender_rx.try_recv().expect("AADC status response");
    let mut response = wow_packet::WorldPacket::from_bytes(&bytes);
    assert_eq!(
        response.read_uint16().expect("opcode"),
        wow_constants::ServerOpcodes::UpdateAadcStatusResponse as u16
    );
    assert!(response.read_bit().expect("success"));
    assert!(!response.read_bit().expect("chat disabled"));
    assert!(response.is_empty());
}

#[tokio::test]
async fn say_uses_cpp_configured_listen_range_like_cpp() {
    let sender = ObjectGuid::create_player(1, 335);
    let nearby = ObjectGuid::create_player(1, 336);
    let far = ObjectGuid::create_player(1, 337);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (far_tx, far_rx) = flume::bounded(8);
    let chat_policy = ChatPolicyCatalogsLikeCpp {
        listen_ranges: ChatListenRangesLikeCpp {
            say: 40.0,
            text_emote: 25.0,
            yell: 300.0,
        },
        ..ChatPolicyCatalogsLikeCpp::default()
    };
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at(
            nearby,
            nearby_tx,
            wow_core::Position::new(30.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    player_registry.register_or_replace(
        far,
        broadcast_info_at(far, far_tx, wow_core::Position::new(41.0, 0.0, 0.0, 0.0)),
        Default::default(),
    );

    session
        .handle_chat_message_with_policy_like_cpp(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "configured say"),
            ChatMsg::Say,
            &chat_policy,
        )
        .await;

    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("sender say echo")),
        "configured say"
    );
    assert_eq!(
        chat_text(&nearby_rx.try_recv().expect("nearby say")),
        "configured say"
    );
    assert!(far_rx.try_recv().is_err());
}

#[tokio::test]
async fn yell_uses_cpp_configured_listen_range_like_cpp() {
    let sender = ObjectGuid::create_player(1, 338);
    let nearby = ObjectGuid::create_player(1, 339);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    session.set_chat_listen_ranges_like_cpp(ChatListenRangesLikeCpp {
        say: 25.0,
        text_emote: 25.0,
        yell: 20.0,
    });
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at(
            nearby,
            nearby_tx,
            wow_core::Position::new(30.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageYell, "configured yell"),
            ChatMsg::Yell,
        )
        .await;

    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("sender yell echo")),
        "configured yell"
    );
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn chat_emote_uses_cpp_configured_text_emote_range_like_cpp() {
    let sender = ObjectGuid::create_player(1, 340);
    let nearby = ObjectGuid::create_player(1, 341);
    let far = ObjectGuid::create_player(1, 342);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (far_tx, far_rx) = flume::bounded(8);
    session.set_chat_listen_ranges_like_cpp(ChatListenRangesLikeCpp {
        say: 25.0,
        text_emote: 40.0,
        yell: 300.0,
    });
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at(
            nearby,
            nearby_tx,
            wow_core::Position::new(30.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    player_registry.register_or_replace(
        far,
        broadcast_info_at(far, far_tx, wow_core::Position::new(41.0, 0.0, 0.0, 0.0)),
        Default::default(),
    );

    session
        .handle_chat_emote(chat_emote_packet("configured emote"))
        .await;

    assert_eq!(
        chat_text(&sender_rx.try_recv().expect("sender emote echo")),
        "configured emote"
    );
    assert_eq!(
        chat_text(&nearby_rx.try_recv().expect("nearby emote")),
        "configured emote"
    );
    assert!(far_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_uses_cpp_configured_text_emote_range_like_cpp() {
    let sender = ObjectGuid::create_player(1, 343);
    let nearby = ObjectGuid::create_player(1, 344);
    let far = ObjectGuid::create_player(1, 345);
    let (mut session, player_registry, _sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(8);
    let (far_tx, far_rx) = flume::bounded(8);
    let (far_command_tx, far_command_rx) = flume::bounded(8);
    session.set_chat_listen_ranges_like_cpp(ChatListenRangesLikeCpp {
        say: 25.0,
        text_emote: 40.0,
        yell: 300.0,
    });
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at_with_command_tx(
            nearby,
            nearby_tx,
            nearby_command_tx,
            wow_core::Position::new(30.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    player_registry.register_or_replace(
        far,
        broadcast_info_at_with_command_tx(
            far,
            far_tx,
            far_command_tx,
            wow_core::Position::new(41.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    set_emotes_text_entries(
        &mut session,
        [emotes_text_entry(66, EMOTE_STATE_SIT_LIKE_CPP as u16)],
    );

    session.handle_text_emote(text_emote_packet(66, 7)).await;

    let nearby_text_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(
        text_emote_fields(&nearby_text_command.packet_bytes),
        (66, 7)
    );
    assert!(nearby_rx.try_recv().is_err());
    assert!(far_rx.try_recv().is_err());
    assert!(nearby_command_rx.try_recv().is_err());
    assert!(far_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_filters_visible_map_instance_like_cpp() {
    let sender = ObjectGuid::create_player(1, 346);
    let target = ObjectGuid::create_player(1, 347);
    let nearby = ObjectGuid::create_player(1, 348);
    let other_instance = ObjectGuid::create_player(1, 349);
    let not_in_world = ObjectGuid::create_player(1, 350);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (target_tx, target_rx) = flume::bounded(8);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(8);
    let (other_instance_tx, other_instance_rx) = flume::bounded(8);
    let (other_instance_command_tx, other_instance_command_rx) = flume::bounded(8);
    let (not_in_world_tx, not_in_world_rx) = flume::bounded(8);
    let (not_in_world_command_tx, not_in_world_command_rx) = flume::bounded(8);
    session.set_chat_listen_ranges_like_cpp(ChatListenRangesLikeCpp {
        say: 25.0,
        text_emote: 40.0,
        yell: 300.0,
    });
    player_registry.register_or_replace(
        target,
        broadcast_info_at(
            target,
            target_tx,
            wow_core::Position::new(60.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at_with_command_tx(
            nearby,
            nearby_tx,
            nearby_command_tx,
            wow_core::Position::new(20.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    let mut other_instance_info = broadcast_info_at_with_command_tx(
        other_instance,
        other_instance_tx,
        other_instance_command_tx,
        wow_core::Position::new(20.0, 0.0, 0.0, 0.0),
    );
    other_instance_info.placement.instance_id = 1;
    player_registry.register_or_replace(other_instance, other_instance_info, Default::default());
    let mut not_in_world_info = broadcast_info_at_with_command_tx(
        not_in_world,
        not_in_world_tx,
        not_in_world_command_tx,
        wow_core::Position::new(20.0, 0.0, 0.0, 0.0),
    );
    not_in_world_info.placement.is_in_world = false;
    player_registry.register_or_replace(not_in_world, not_in_world_info, Default::default());
    set_emotes_text_entries(
        &mut session,
        [emotes_text_entry(66, EMOTE_STATE_SIT_LIKE_CPP as u16)],
    );

    session
        .handle_text_emote(text_emote_packet_with_target(target, 66, 7))
        .await;

    assert_eq!(
        text_emote_fields_with_target(&sender_rx.try_recv().expect("sender text emote")),
        (66, 7, target)
    );
    let nearby_text_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(
        text_emote_fields_with_target(&nearby_text_command.packet_bytes),
        (66, 7, target)
    );
    assert!(nearby_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
    assert!(other_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
    assert!(nearby_command_rx.try_recv().is_err());
    assert!(other_instance_command_rx.try_recv().is_err());
    assert!(not_in_world_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_requires_cpp_emotes_text_entry_like_cpp() {
    let sender = ObjectGuid::create_player(1, 351);
    let nearby = ObjectGuid::create_player(1, 352);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    set_emotes_text_entries(&mut session, Vec::<wow_data::EmotesTextEntry>::new());

    session.handle_text_emote(text_emote_packet(66, 7)).await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}

#[tokio::test]
async fn emote_client_does_not_publish_noop_clear_like_cpp() {
    let sender = ObjectGuid::create_player(1, 355);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_emote(wow_packet::WorldPacket::new_empty())
        .await;

    assert_eq!(session.player_emote_state_like_cpp(), 0);
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_translates_emotes_text_and_uses_cpp_order_and_ranges_like_cpp() {
    let sender = ObjectGuid::create_player(1, 348);
    let nearby = ObjectGuid::create_player(1, 349);
    let far = ObjectGuid::create_player(1, 350);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(8);
    let (far_tx, far_rx) = flume::bounded(8);
    let (far_command_tx, far_command_rx) = flume::bounded(8);
    session.set_chat_listen_ranges_like_cpp(ChatListenRangesLikeCpp {
        say: 25.0,
        text_emote: 40.0,
        yell: 300.0,
    });
    player_registry.register_or_replace(
        nearby,
        broadcast_info_at_with_command_tx(
            nearby,
            nearby_tx,
            nearby_command_tx,
            wow_core::Position::new(30.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    player_registry.register_or_replace(
        far,
        broadcast_info_at_with_command_tx(
            far,
            far_tx,
            far_command_tx,
            wow_core::Position::new(60.0, 0.0, 0.0, 0.0),
        ),
        Default::default(),
    );
    set_emotes_text_entries(&mut session, [emotes_text_entry(66, 3)]);

    session
        .handle_text_emote(text_emote_packet_with_visuals(66, 7, &[101, 202], 9))
        .await;

    assert_eq!(
        emote_message_fields(&sender_rx.try_recv().expect("sender anim emote")),
        (3, Vec::new(), 9)
    );
    assert_eq!(
        text_emote_fields(&sender_rx.try_recv().expect("sender text emote")),
        (66, 7)
    );
    let nearby_anim_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(
        emote_message_fields(&nearby_anim_command.packet_bytes),
        (3, Vec::new(), 9)
    );
    let nearby_text_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(
        text_emote_fields(&nearby_text_command.packet_bytes),
        (66, 7)
    );
    let far_anim_command = expect_send_if_visible_command(&far_command_rx);
    assert_eq!(
        emote_message_fields(&far_anim_command.packet_bytes),
        (3, Vec::new(), 9)
    );
    assert!(nearby_rx.try_recv().is_err());
    assert!(far_rx.try_recv().is_err());
    assert!(nearby_command_rx.try_recv().is_err());
    assert!(far_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_keeps_spell_visual_kits_only_for_cpp_mount_special_like_cpp() {
    let sender = ObjectGuid::create_player(1, 351);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    set_emotes_text_entries(&mut session, [emotes_text_entry(77, 555)]);
    set_emotes_entries(
        &mut session,
        [emotes_entry(555, ANIM_MOUNT_SPECIAL_LIKE_CPP)],
    );

    session
        .handle_text_emote(text_emote_packet_with_visuals(77, 8, &[11, 22], 4))
        .await;

    assert_eq!(
        emote_message_fields(&sender_rx.try_recv().expect("sender anim emote")),
        (555, vec![11, 22], 4)
    );
    assert_eq!(
        text_emote_fields(&sender_rx.try_recv().expect("sender text emote")),
        (77, 8)
    );
}

#[tokio::test]
async fn send_text_emote_fake_death_skips_animation_but_keeps_text_like_cpp() {
    let sender = ObjectGuid::create_player(1, 352);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let canonical = bind_canonical_player_like_cpp(&player_registry, sender, |player| {
        player.unit_mut().add_unit_state(UnitState::DIED.bits());
    });
    session.set_canonical_map_manager(canonical);
    set_emotes_text_entries(&mut session, [emotes_text_entry(66, 3)]);

    session.handle_text_emote(text_emote_packet(66, 7)).await;

    assert_eq!(
        text_emote_fields(&sender_rx.try_recv().expect("sender text emote")),
        (66, 7)
    );
    assert!(sender_rx.try_recv().is_err());
}

#[tokio::test]
async fn send_text_emote_dance_publishes_emote_state_update_like_cpp() {
    let sender = ObjectGuid::create_player(1, 353);
    let nearby = ObjectGuid::create_player(1, 354);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    let (nearby_command_tx, nearby_command_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info_with_command_tx(nearby, nearby_tx, nearby_command_tx),
        Default::default(),
    );
    set_emotes_text_entries(
        &mut session,
        [emotes_text_entry(34, EMOTE_STATE_DANCE_LIKE_CPP as u16)],
    );

    session.handle_text_emote(text_emote_packet(34, -1)).await;

    let mut sender_update =
        wow_packet::WorldPacket::from_bytes(&sender_rx.try_recv().expect("sender update"));
    assert_eq!(
        sender_update.read_uint16().expect("sender update opcode"),
        wow_constants::ServerOpcodes::UpdateObject as u16
    );
    assert_eq!(
        text_emote_fields(&sender_rx.try_recv().expect("sender text emote")),
        (34, -1)
    );
    assert!(sender_rx.try_recv().is_err());

    let nearby_update_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(nearby_update_command.source_guid, sender);
    assert_eq!(nearby_update_command.map_id, 571);
    assert_eq!(nearby_update_command.instance_id, 0);
    let mut nearby_update =
        wow_packet::WorldPacket::from_bytes(&nearby_update_command.packet_bytes);
    assert_eq!(
        nearby_update.read_uint16().expect("nearby update opcode"),
        wow_constants::ServerOpcodes::UpdateObject as u16
    );
    let nearby_text_command = expect_send_if_visible_command(&nearby_command_rx);
    assert_eq!(nearby_text_command.source_guid, sender);
    assert_eq!(nearby_text_command.map_id, 571);
    assert_eq!(nearby_text_command.instance_id, 0);
    assert_eq!(
        text_emote_fields(&nearby_text_command.packet_bytes),
        (34, -1)
    );
    assert!(nearby_rx.try_recv().is_err());
    assert!(nearby_command_rx.try_recv().is_err());
}

#[tokio::test]
async fn officer_chat_does_not_leak_to_nearby_players_without_guild_registry_like_cpp() {
    let sender = ObjectGuid::create_player(1, 311);
    let nearby = ObjectGuid::create_player(1, 312);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageOfficer, "officer"),
            ChatMsg::Officer,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
    assert!(!session.is_disconnecting());
}

#[tokio::test]
async fn chat_strict_link_checking_kick_disconnects_on_invalid_link_like_cpp() {
    let sender = ObjectGuid::create_player(1, 353);
    let nearby = ObjectGuid::create_player(1, 354);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );
    let chat_policy = ChatPolicyCatalogsLikeCpp {
        strict_link_checking_kick: true,
        ..ChatPolicyCatalogsLikeCpp::default()
    };

    session
        .handle_chat_message_with_policy_like_cpp(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "forged |x control"),
            ChatMsg::Say,
            &chat_policy,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
    assert!(session.is_disconnecting());
}

#[tokio::test]
async fn chat_dispatch_borrows_process_policy_instead_of_session_copy_like_cpp() {
    let sender = ObjectGuid::create_player(1, 355);
    let (mut session, _, sender_rx) = session_for_chat_routing_like_cpp(sender);
    session.set_state(crate::session::SessionState::LoggedIn);
    let catalogs = crate::session::SessionHandlerCatalogsLikeCpp {
        chat_policy: Arc::new(ChatPolicyCatalogsLikeCpp {
            strict_link_checking_kick: true,
            ..ChatPolicyCatalogsLikeCpp::default()
        }),
        ..crate::session::SessionHandlerCatalogsLikeCpp::default()
    };
    let text = "forged |x control";
    let mut writer = wow_packet::WorldPacket::new_empty();
    writer.write_uint16(ClientOpcodes::ChatMessageSay as u16);
    writer.write_int32(LANG_COMMON_LIKE_CPP);
    writer.write_bits(text.len() as u32, 11);
    writer.write_bit(false);
    writer.write_string(text);

    session
        .dispatch_packet(
            &catalogs,
            wow_packet::WorldPacket::from_bytes(writer.data()),
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(session.is_disconnecting());
}

#[tokio::test]
async fn chat_message_truncates_at_newline_like_cpp() {
    let sender = ObjectGuid::create_player(1, 331);
    let (mut session, _player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "visible\nhidden"),
            ChatMsg::Say,
        )
        .await;

    let delivered = sender_rx.try_recv().expect("sender echo");
    assert_eq!(chat_text(&delivered), "visible");
}

#[tokio::test]
async fn chat_with_invalid_hyperlink_control_sequence_is_rejected_like_cpp() {
    let sender = ObjectGuid::create_player(1, 351);
    let nearby = ObjectGuid::create_player(1, 352);
    let (mut session, player_registry, sender_rx) = session_for_chat_routing_like_cpp(sender);
    let (nearby_tx, nearby_rx) = flume::bounded(8);
    player_registry.register_or_replace(
        nearby,
        broadcast_info(nearby, nearby_tx),
        Default::default(),
    );

    session
        .handle_chat_message(
            chat_message_packet(ClientOpcodes::ChatMessageSay, "forged |x control"),
            ChatMsg::Say,
        )
        .await;

    assert!(sender_rx.try_recv().is_err());
    assert!(nearby_rx.try_recv().is_err());
}
