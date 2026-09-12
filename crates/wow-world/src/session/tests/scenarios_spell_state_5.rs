//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn visible_unit_values_update_command_applies_viewer_dependent_spellclick_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(127);

    session.state = SessionState::LoggedIn;
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [],
        |entry| entry == 707,
        |_| false,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        707,
        Position::new(12.0, 0.0, 0.0, 0.0),
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    let mut packet_update = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
    packet_update.changed_object_type_mask = 1 << TYPEID_UNIT;
    packet_update.unit_data_mask[113 / 32] |= 1 << (113 % 32);
    packet_update.unit_data_mask[114 / 32] |= 1 << (114 % 32);
    packet_update.npc_flags = [
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
        0,
    ];
    let raw_packet = wow_packet::packets::update::UpdateObject::unit_values_update(
        creature_guid,
        571,
        packet_update.clone(),
    )
    .to_bytes();
    let expected_packet = session
        .represented_unit_packet_update_to_update_object_like_cpp(
            creature_guid,
            571,
            packet_update.clone(),
        )
        .to_bytes();
    assert_ne!(raw_packet, expected_packet);

    session
        .session_command_tx()
        .try_send(SessionCommand::SendVisibleObjectValuesUpdate(
            SendVisibleObjectValuesUpdateCommand {
                object_guid: creature_guid,
                map_id: 571,
                packet_bytes: raw_packet,
                unit_values_update: Some(packet_update),
            },
        ))
        .expect("command queued");
    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(
        send_rx.try_recv().expect("filtered visible unit update"),
        expected_packet
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn update_visible_spell_clicks_sends_conditioned_npcflags_delta_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(128);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 708,
            source_entry: 908,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 708,
            spell_id: 908,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 708,
        |spell| spell == 908,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        708,
        Position::new(12.0, 0.0, 0.0, 0.0),
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    let mut packet_update = wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
    packet_update.changed_object_type_mask = 1 << TYPEID_UNIT;
    packet_update.unit_data_mask[113 / 32] |= 1 << (113 % 32);
    packet_update.unit_data_mask[114 / 32] |= 1 << (114 % 32);
    packet_update.npc_flags = [
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
        0,
    ];
    let expected_packet = session
        .represented_unit_packet_update_to_update_object_like_cpp(creature_guid, 571, packet_update)
        .to_bytes();

    assert_eq!(session.update_visible_spell_clicks_like_cpp(), 1);
    assert_eq!(
        send_rx.try_recv().expect("spellclick refresh update"),
        expected_packet
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn update_visible_spell_clicks_skips_unconditioned_spellclick_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(129);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 709,
            spell_id: 909,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 709,
        |spell| spell == 909,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        709,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    assert_eq!(session.update_visible_spell_clicks_like_cpp(), 0);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn complete_quest_triggers_visible_spellclick_refresh_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(130);
    let quest_id = 12_540;
    let mut quest = test_quest_template(quest_id);
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: QUEST_OBJECTIVE_MONEY_LIKE_CPP,
        order: 0,
        storage_index: -1,
        object_id: 0,
        amount: 100,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });

    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical spell-click viewer map");
    session.set_player_gold_like_cpp(90);
    assert!(
        session
            .mutate_player_quest_gameplay_like_cpp(|state| {
                state.insert_status_like_cpp(
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
            })
            .is_some(),
        "quest fixture must be loaded into the canonical Player owner"
    );
    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 710,
            source_entry: 910,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 710,
            spell_id: 910,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 710,
        |spell| spell == 910,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        710,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    session.money_changed_like_cpp(100).await;

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(update_object_packet_count_like_cpp(&packets), 1);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
}
#[tokio::test]
async fn leave_group_triggers_visible_spellclick_refresh_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let creature_guid = test_creature_guid(131);
    let (other_tx, _other_rx) = flume::bounded(8);
    let player_registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    player_registry.register_or_replace(
        other_guid,
        broadcast_info(other_guid, other_tx),
        Default::default(),
    );
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(other_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::SpellClickEvent,
            source_group: 711,
            source_entry: 911,
            source_id: 0,
            condition_type: ConditionType::Aura,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 711,
            spell_id: 911,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 711,
        |spell| spell == 911,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        711,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session.client_visible_guids_like_cpp.insert(creature_guid);

    let mut pkt = wow_packet::WorldPacket::new_empty();
    pkt.write_bit(false);
    pkt.flush_bits();
    pkt.reset_read();
    session.handle_leave_group(pkt).await;

    assert_eq!(session.group_guid, None);
    let packets = drain_server_packet_bytes(&send_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupUninvite)
    }));
}
#[test]
fn represented_mount_aura_display_candidates_match_cpp_filter() {
    let (mut session, _, _) = make_session();
    session.player_class = 1;
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 7,
            mount_type_id: 0,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
        wow_data::MountEntry {
            id: 8,
            mount_type_id: 0,
            flags: wow_data::MOUNT_FLAG_SELF_MOUNT,
            source_type_enum: 0,
            source_spell_id: 101,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_x_display_store(Arc::new(wow_data::MountXDisplayStore::from_entries([
        wow_data::MountXDisplayEntry {
            id: 1,
            creature_display_info_id: 1000,
            player_condition_id: 42,
            mount_id: 7,
        },
        wow_data::MountXDisplayEntry {
            id: 2,
            creature_display_info_id: 1001,
            player_condition_id: 43,
            mount_id: 7,
        },
        wow_data::MountXDisplayEntry {
            id: 3,
            creature_display_info_id: 1002,
            player_condition_id: 0,
            mount_id: 7,
        },
    ])));
    session.set_player_condition_store(Arc::new(wow_data::PlayerConditionStore::from_entries([
        wow_data::PlayerConditionEntry {
            id: 42,
            class_mask: 1,
            ..Default::default()
        },
        wow_data::PlayerConditionEntry {
            id: 43,
            class_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    assert_eq!(
        session.represented_mount_aura_display_candidates_like_cpp(100),
        vec![1000, 1002]
    );
    assert_eq!(
        session.represented_mount_aura_display_candidates_like_cpp(101),
        vec![wow_data::DISPLAYID_HIDDEN_MOUNT]
    );
    assert!(
        session
            .represented_mount_aura_display_candidates_like_cpp(999)
            .is_empty()
    );
}
#[test]
fn summon_object_wild_session_resolver_uses_caster_close_point_when_dst_missing_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 700_u32;
    let template_entry = 9001_u32;
    let player_guid = ObjectGuid::create_player(1, 7002);
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 20.0, 30.0, 0.0);
    insert_test_player_into_canonical_map_like_cpp(
        &canonical,
        player_guid,
        571,
        0,
        player_position,
    );
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(530, player_position);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session
        .set_gameobject_template_lifecycle_store(summon_go_template_store_like_cpp(template_entry));

    let outcome = session
        .apply_effect_summon_object_wild_like_cpp(
            i32::try_from(spell_id).unwrap(),
            &summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap()),
            &SpellTargetData::default(),
        )
        .expect("wild effect should return represented outcome");

    assert_eq!(
        outcome.status,
        ApplyEffectSummonObjectWildSessionStatusLikeCpp::MapResolved
    );
    assert!(!outcome.explicit_destination_used);
    assert!(outcome.close_point_fallback_represented);
    let map_outcome = outcome.map_outcome.expect("map body should run");
    let go_guid = map_outcome.guid.expect("created GO guid");
    let manager = canonical.lock().unwrap();
    let go = manager
        .find_map(571, 0)
        .and_then(|managed| managed.map().get_typed_game_object(go_guid))
        .expect("summoned GO should be in canonical map");
    assert_eq!(
        go.world().position(),
        Position::new(
            10.0 + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP,
            20.0,
            30.0,
            0.0
        )
    );
}
#[tokio::test]
async fn summon_object_slot_live_spell_without_focus_creates_visible_slotted_go_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 703_i32;
    let template_entry = 9004_u32;
    let player_guid = ObjectGuid::create_player(1, 7005);
    let player_position = Position::new(120.0, 220.0, 32.0, 0.0);
    let canonical = shared_canonical_map_manager();
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(
            spell_id,
            0,
            vec![summon_object_slot_effect_like_cpp(
                i32::try_from(template_entry).unwrap(),
                0,
            )],
        ),
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 703,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("live non-focus slotted GameObject summon should execute");

    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .find(ObjectGuid::is_game_object)
        .expect("force visibility should make the newly slotted GameObject visible");
    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let owner = managed
        .map()
        .get_typed_player(player_guid)
        .expect("player remains slot owner");
    assert_eq!(
        owner.unit().subsystems().control.gameobject_slots[0],
        summoned_guid
    );
    let gameobject = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("visible slotted GO should be map-owned");
    assert_eq!(gameobject.world().object().entry(), template_entry);
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "live slotted summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn summon_object_wild_live_spell_with_focus_uses_focus_orientation_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 705_i32;
    let template_entry = 9006_u32;
    let player_guid = ObjectGuid::create_player(1, 7008);
    let focus_guid = test_gameobject_guid(9007, 7007);
    let player_position = Position::new(160.0, 260.0, 36.0, 0.0);
    let focus_position = Position::new(162.0, 260.0, 36.0, 1.25);
    let canonical = shared_canonical_map_manager();
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(
            spell_id,
            181,
            vec![summon_object_wild_effect_like_cpp(
                i32::try_from(template_entry).unwrap(),
            )],
        ),
    );
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        focus_guid,
        9_007,
        181,
        10,
        focus_position,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 705,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("focus-backed wild GameObject summon should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("summoned focus-backed GO should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        Position::new(
            focus_position.x
                + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP
                    * focus_position.orientation.cos(),
            focus_position.y
                + wow_map::map::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP
                    * focus_position.orientation.sin(),
            focus_position.z,
            focus_position.orientation,
        )
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "focus-backed summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn focus_implicit_destination_uses_effect_facing_when_spell_attr4_requests_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 710_i32;
    let template_entry = 9014_u32;
    let player_guid = ObjectGuid::create_player(1, 7014);
    let focus_guid = test_gameobject_guid(9015, 7015);
    let player_position = Position::new(200.0, 300.0, 40.0, 0.0);
    let focus_position = Position::new(204.0, 302.0, 40.5, 2.8);
    let effect_position_facing = 0.625;
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    summon_effect.position_facing = effect_position_facing;
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 181, vec![summon_effect]),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.attributes[4] = wow_data::spell::attributes::SPELL_ATTR4_USE_FACING_FROM_SPELL as i32;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    add_canonical_spell_focus_gameobject_on_map_like_cpp(
        &canonical,
        focus_guid,
        9_015,
        181,
        10,
        focus_position,
        571,
        0,
    );

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 710,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("focus implicit destination with facing attr should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("focus-destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        Position::new(
            focus_position.x,
            focus_position.y,
            focus_position.z,
            effect_position_facing
        ),
        "C++ SPELL_ATTR4_USE_FACING_FROM_SPELL overrides focusObject destination orientation with SpellEffectInfo::PositionFacing"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "focus-destination summon should trigger represented visibility create/update delivery"
    );
}
#[tokio::test]
async fn db_implicit_destination_or_db_uses_spell_target_position_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 711_i32;
    let template_entry = 9016_u32;
    let player_guid = ObjectGuid::create_player(1, 7016);
    let player_position = Position::new(200.0, 300.0, 40.0, 0.0);
    let db_destination = Position::new(206.0, 304.0, 41.0, 1.375);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 =
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB;
    let spell_info =
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect.clone()]);

    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        spell_info.clone(),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 88;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 88,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [50.0, 50.0],
        },
    ])));
    let mut target_spell_store = wow_data::SpellStore::new();
    target_spell_store.insert(spell_id, spell_info);
    session.set_spell_target_position_store(Arc::new(
        wow_data::SpellTargetPositionStoreLikeCpp::from_rows_like_cpp(
            [wow_data::SpellTargetPositionRowLikeCpp {
                spell_id: spell_id as u32,
                effect_index: summon_effect.effect_index,
                target_map_id: 571,
                x: db_destination.x,
                y: db_destination.y,
                z: db_destination.z,
                orientation: Some(db_destination.orientation),
            }],
            &target_spell_store,
            |map_id| map_id == 571,
        ),
    ));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 711,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("OR_DB implicit destination with DB target should execute");

    let manager = canonical.lock().unwrap();
    let managed = manager.find_map(571, 0).expect("canonical map");
    let summoned_guid = session
        .client_visible_guids_like_cpp
        .snapshot_like_cpp()
        .into_iter()
        .filter(ObjectGuid::is_game_object)
        .find(|guid| {
            managed
                .map()
                .get_typed_game_object(*guid)
                .is_some_and(|go| go.world().object().entry() == template_entry)
        })
        .expect("DB-destination summon should be visible");
    let summoned = managed
        .map()
        .get_typed_game_object(summoned_guid)
        .expect("summoned GO should be map-owned");
    assert_eq!(
        summoned.world().position(),
        db_destination,
        "C++ TARGET_DEST_NEARBY_ENTRY_OR_DB uses spell_target_position when same-map and in range"
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert!(
        update_object_packet_count_like_cpp(&packets) >= 1,
        "DB-destination summon should trigger represented visibility create/update delivery"
    );
}
