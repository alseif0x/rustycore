//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_force_deselect_effect_row_records_break_and_clear_packets_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 755_i32;
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_DESELECT,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented force-deselect spell row should execute");

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent],
        "C++ sends break/clear to hostile visible clients, not directly to the caster session"
    );
    let evidence = session
        .represented_force_deselects_like_cpp()
        .first()
        .expect("represented force-deselect evidence");
    assert_eq!(evidence.caster_guid, player_guid);
    assert_eq!(evidence.visibility_range_yards, 100);
    assert!(evidence.hostile_visible_fanout_unrepresented);
    assert!(evidence.attacker_pet_attack_stop_unrepresented);

    let mut break_target = wow_packet::WorldPacket::from_bytes(&evidence.break_target_packet_bytes);
    assert_eq!(
        break_target.read_uint16().expect("break opcode"),
        ServerOpcodes::BreakTarget as u16
    );
    assert_eq!(
        break_target.read_packed_guid().expect("UnitGUID"),
        player_guid
    );
    assert!(break_target.is_empty());

    let mut clear_target = wow_packet::WorldPacket::from_bytes(&evidence.clear_target_packet_bytes);
    assert_eq!(
        clear_target.read_uint16().expect("clear opcode"),
        ServerOpcodes::ClearTarget as u16
    );
    assert_eq!(clear_target.read_packed_guid().expect("Guid"), player_guid);
    assert!(clear_target.is_empty());
}
#[tokio::test]
async fn spell_change_raid_marker_effect_row_stores_marker_and_fanouts_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 756_i32;
    let leader_guid = ObjectGuid::create_player(1, 78);
    let member_guid = ObjectGuid::create_player(1, 79);
    let destination = Position::xyz(12.25, -34.5, 6.75);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = flume::bounded(8);
    let (member_tx, member_rx) = flume::bounded(8);
    let mut group = GroupInfo::new(leader_guid);
    group.convert_to_raid_like_cpp();
    group.add_member(member_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        leader_guid,
        broadcast_info(leader_guid, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_guid(Some(leader_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER,
                effect_base_points: 3,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_target_data(
            spell_id,
            leader_guid,
            SpellTargetData {
                dst_location: Some(wow_packet::packets::spell::TargetLocation {
                    transport: ObjectGuid::EMPTY,
                    position: destination,
                }),
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented change-raid-marker spell row should execute");

    let group = group_registry
        .get(&group_guid)
        .expect("group should remain");
    assert_eq!(group.active_raid_markers_mask_like_cpp(), 1 << 3);
    assert_eq!(group.raid_marker_list_like_cpp()[0].map_id, 571);
    assert_eq!(group.raid_marker_list_like_cpp()[0].position, destination);
    drop(group);

    for rx in [&leader_rx, &member_rx] {
        let marker_packet = rx.try_recv().expect("raid marker fanout packet");
        let mut packet = WorldPacket::from_bytes(&marker_packet);
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::RaidMarkersChanged)
        );
        assert_eq!(
            packet.read_uint16().expect("opcode"),
            ServerOpcodes::RaidMarkersChanged as u16
        );
        assert_eq!(packet.read_uint8().expect("PartyIndex"), 0);
        assert_eq!(packet.read_uint32().expect("ActiveMarkers"), 1 << 3);
        assert_eq!(packet.read_bits(4).expect("marker count"), 1);
        packet.flush_bits();
        assert_eq!(
            packet.read_packed_guid().expect("TransportGUID"),
            ObjectGuid::EMPTY
        );
        assert_eq!(packet.read_uint32().expect("mapId"), 571);
        assert_eq!(packet.read_float().expect("x"), destination.x);
        assert_eq!(packet.read_float().expect("y"), destination.y);
        assert_eq!(packet.read_float().expect("z"), destination.z);
        assert!(packet.is_empty());
    }
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent],
        "RaidMarkersChanged is sent to represented group members, not as an extra direct caster packet"
    );
}
#[tokio::test]
async fn spell_change_raid_marker_raid_requires_leader_or_assistant_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 757_i32;
    let leader_guid = ObjectGuid::create_player(1, 80);
    let member_guid = ObjectGuid::create_player(1, 81);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::default());
    let (member_tx, member_rx) = flume::bounded(8);
    let mut group = GroupInfo::new(leader_guid);
    group.convert_to_raid_like_cpp();
    group.add_member(member_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        member_guid,
        broadcast_info(member_guid, member_tx),
        Default::default(),
    );
    session.set_player_guid(Some(member_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER,
                effect_base_points: 4,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_target_data(
            spell_id,
            member_guid,
            SpellTargetData {
                dst_location: Some(wow_packet::packets::spell::TargetLocation {
                    transport: ObjectGuid::EMPTY,
                    position: Position::xyz(1.0, 2.0, 3.0),
                }),
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented change-raid-marker unauthorized raid member should no-op");

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        0
    );
    assert!(member_rx.try_recv().is_err());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_quest_complete_effect_marks_active_event_quest_complete_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 745_i32;
    let player_guid = ObjectGuid::create_player(1, 62);
    let quest_id = 12_542;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0002; // C++ QUEST_FLAGS_COMPLETION_EVENT.
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE,
                effect_misc_value_1: quest_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented quest-complete spell row should execute");

    let status = session
        .player_quests
        .get(&quest_id)
        .expect("non-tracking quest remains in log");
    assert!(status.explored);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
    );
    assert!(!session.rewarded_quests.contains(&quest_id));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_quest_complete_effect_auto_rewards_active_tracking_event_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 746_i32;
    let player_guid = ObjectGuid::create_player(1, 63);
    let quest_id = 12_543;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0002; // C++ QUEST_FLAGS_COMPLETION_EVENT.
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE,
                effect_misc_value_1: quest_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented tracking quest-complete spell row should execute");

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_quest_complete_effect_rewards_unlogged_tracking_event_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 747_i32;
    let player_guid = ObjectGuid::create_player(1, 64);
    let quest_id = 12_544;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.set_quest_v2_store(Arc::new(QuestV2Store::from_entries([
        wow_data::progression_rewards::QuestV2Entry {
            id: quest_id,
            unique_bit_flag: 65,
        },
    ])));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE,
                effect_misc_value_1: quest_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented unlogged tracking quest-complete spell row should execute");

    assert!(session.rewarded_quests.contains(&quest_id));
    assert!(
        session
            .represented_quest_completed_bits_like_cpp
            .contains(&65)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_quest_complete_effect_keeps_failed_active_quest_unchanged_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 748_i32;
    let player_guid = ObjectGuid::create_player(1, 65);
    let quest_id = 12_545;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0002; // C++ QUEST_FLAGS_COMPLETION_EVENT.
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE,
                effect_misc_value_1: quest_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented failed quest-complete spell row should execute as no-op");

    let status = session.player_quests.get(&quest_id).expect("quest remains");
    assert!(!status.explored);
    assert_eq!(
        status.status,
        crate::conditions::QUEST_STATUS_FAILED_LIKE_CPP
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_cpp_unused_effect_row_is_represented_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 749_i32;
    let player_guid = ObjectGuid::create_player(1, 66);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(77, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: [
                wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_COMBO_POINTS,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PET,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_122,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_FRIEND,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_ENEMY,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_OWNER,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_178,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_194,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_209,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_235,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_241,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_256,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_257,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_262,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_274,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_275,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_280,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_300,
            ]
            .into_iter()
            .enumerate()
            .map(|(effect_index, effect)| wow_data::SpellEffectInfo {
                effect_index: effect_index as u32,
                effect,
                ..Default::default()
            })
            .collect(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented C++ EffectUnused row should no-op");

    assert_eq!(session.player_health_like_cpp(), 77);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_play_movie_sends_trigger_movie_for_valid_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 68);
    let spell_id = 863_i32;
    let movie_id = 177_u32;
    session.set_player_guid(Some(player_guid));
    session.set_movie_store(Arc::new(wow_data::MovieStore::from_entries([
        wow_data::MovieEntry {
            id: movie_id,
            volume: 0,
            key_id: 0,
            audio_file_data_id: 0,
            subtitle_file_data_id: 0,
        },
    ])));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE,
                effect_misc_value_1: movie_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented C++ EffectPlayMovie should execute");

    assert_eq!(session.represented_movie_like_cpp(), Some(movie_id));
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 3);
    assert_eq!(
        packets[0][0..2],
        (ServerOpcodes::SpellGo as u16).to_le_bytes()
    );
    assert_eq!(
        packets[1][0..2],
        (ServerOpcodes::TriggerMovie as u16).to_le_bytes()
    );
    assert_eq!(&packets[1][2..6], &movie_id.to_le_bytes());
    assert_eq!(packets[1].len(), 6);
    assert_eq!(
        packets[2][0..2],
        (ServerOpcodes::CooldownEvent as u16).to_le_bytes()
    );
}
#[tokio::test]
async fn spell_play_movie_skips_missing_movie_or_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 69);
    let other_player_guid = ObjectGuid::create_player(1, 70);
    let spell_id = 864_i32;
    session.set_player_guid(Some(player_guid));
    session.set_movie_store(Arc::new(wow_data::MovieStore::from_entries([
        wow_data::MovieEntry {
            id: 55,
            volume: 0,
            key_id: 0,
            audio_file_data_id: 0,
            subtitle_file_data_id: 0,
        },
    ])));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, movie_id) in [(spell_id, 99_i32), (spell_id + 1, -1_i32)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE,
                    effect_misc_value_1: movie_id,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    for (cast_spell, target_guid) in [
        (spell_id, player_guid),
        (spell_id + 1, player_guid),
        (spell_id, other_player_guid),
    ] {
        session
            .execute_spell(cast_spell, target_guid)
            .await
            .expect("represented C++ EffectPlayMovie guard should no-op");
        assert_eq!(session.represented_movie_like_cpp(), None);
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
        );
    }
}
#[tokio::test]
async fn spell_cpp_empty_real_handler_is_represented_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 68);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(91, 100);

    let hydrated_spell_id = 857_i32;
    let primary_spell_id = 858_i32;
    let pull_hydrated_spell_id = 859_i32;
    let pull_primary_spell_id = 860_i32;
    let trade_skill_hydrated_spell_id = 861_i32;
    let trade_skill_primary_spell_id = 862_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        hydrated_spell_id,
        wow_data::SpellInfo {
            spell_id: hydrated_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        primary_spell_id,
        wow_data::SpellInfo {
            spell_id: primary_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    spell_store.insert(
        pull_hydrated_spell_id,
        wow_data::SpellInfo {
            spell_id: pull_hydrated_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL,
                effect_base_points: 30,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        pull_primary_spell_id,
        wow_data::SpellInfo {
            spell_id: pull_primary_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL,
            effect_base_points: 30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    spell_store.insert(
        trade_skill_hydrated_spell_id,
        wow_data::SpellInfo {
            spell_id: trade_skill_hydrated_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 75,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL,
                effect_misc_value_1: 164,
                effect_base_points: 75,
                ..Default::default()
            }],
        },
    );
    spell_store.insert(
        trade_skill_primary_spell_id,
        wow_data::SpellInfo {
            spell_id: trade_skill_primary_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL,
            effect_base_points: 75,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    for spell_id in [
        hydrated_spell_id,
        primary_spell_id,
        pull_hydrated_spell_id,
        pull_primary_spell_id,
        trade_skill_hydrated_spell_id,
        trade_skill_primary_spell_id,
    ] {
        session
            .execute_spell(spell_id, player_guid)
            .await
            .expect("C++ real no-op spell handler should no-op");
        assert_eq!(session.player_health_like_cpp(), 91);
        assert_eq!(
            drain_server_opcodes(&send_rx),
            vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
        );
    }
}
