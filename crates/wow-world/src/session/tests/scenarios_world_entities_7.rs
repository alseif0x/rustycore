//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn visible_gameobjects_create_uses_per_player_go_state_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 515);
    let gameobject_guid = test_gameobject_guid(914, 914);

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Viewer".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        571,
        2,
        1,
        10,
        0,
    ));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(571, 0);

    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        914,
        Position::new(20.0, 20.0, 0.0, 0.0),
        wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        gameobject_guid,
        7_014,
        1.0,
        [0.0, 0.0, 0.0, 1.0],
    );
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&gameobject_guid)
            .expect("represented state");
        state.go_state = Some(wow_entities::GoState::Ready);
        state.per_player_state_player_guid = Some(player_guid);
        state.per_player_go_state = Some(wow_entities::GoState::Active);
        state.per_player_go_state_until = Some(Instant::now() + Duration::from_secs(30));
    }

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(
            571,
            &Position::new(10.0, 10.0, 0.0, 0.0),
            800.0,
        )
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, gameobject_guid);
    assert_eq!(visible[0].state, wow_entities::GoState::Active as i8);
    assert_eq!(
        visible[0].dynamic_flags,
        wow_entities::GO_DYNFLAG_LO_DEPLETED,
        "C++ GameObjectData::DynamicFlags create uses viewer-dependent GetGoStateFor"
    );

    session
        .represented_gameobject_use_states
        .get_mut(&gameobject_guid)
        .expect("represented state")
        .per_player_go_state_until = Some(Instant::now() - Duration::from_secs(1));

    let visible_after_expiry = session
        .visible_gameobjects_from_canonical_map_like_cpp(
            571,
            &Position::new(10.0, 10.0, 0.0, 0.0),
            800.0,
        )
        .expect("canonical map");

    assert_eq!(visible_after_expiry.len(), 1);
    assert_eq!(
        visible_after_expiry[0].state,
        wow_entities::GoState::Ready as i8,
        "expired Rust per-player state must not override C++ GetGoState fallback"
    );
    assert_eq!(visible_after_expiry[0].dynamic_flags, 0);
}
#[test]
fn visible_gameobjects_falls_back_to_typed_canonical_data_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let gameobject_guid = test_gameobject_guid(49_630, 49_630);
    let rotation = [0.125, 0.25, 0.375, 0.875];

    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_lifecycle_gameobject_on_map(
        &canonical,
        gameobject_guid,
        49_630,
        Position::new(20.0, 20.0, 0.0, 0.0),
        rotation,
        571,
        0,
    );

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &player_position, 800.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, gameobject_guid);
    assert_eq!(visible[0].entry, 49_630);
    assert_eq!(visible[0].display_id, 7_777);
    assert_eq!(
        visible[0].go_type,
        wow_entities::GAMEOBJECT_TYPE_CHEST as u8
    );
    assert_eq!(visible[0].rotation, rotation);
    assert_eq!(visible[0].anim_progress, 33);
    assert_eq!(visible[0].state, wow_entities::GoState::Ready as i8);
    assert_eq!(
        visible[0].art_kit, 4,
        "canonical GameObjectData::ArtKit must reach later viewers' CREATE blocks"
    );
    assert_eq!(visible[0].faction_template, 35);
    assert_eq!(visible[0].gameobject_flags, 0x24);
    assert_eq!(visible[0].scale, 1.75);
    assert!(
        !session
            .represented_gameobject_use_states
            .contains_key(&gameobject_guid),
        "C++ AddToMap-visible GameObjects are real map objects; visibility must not require session-local represented state"
    );
}
#[test]
fn visible_gameobjects_skip_not_in_world_canonical_objects_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let in_world_guid = test_gameobject_guid(49_632, 49_632);
    let removed_guid = test_gameobject_guid(49_633, 49_633);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_lifecycle_gameobject_on_map(
        &canonical,
        in_world_guid,
        49_632,
        Position::new(20.0, 20.0, 0.0, 0.0),
        [0.0, 0.0, 0.0, 1.0],
        571,
        0,
    );
    add_canonical_lifecycle_gameobject_on_map(
        &canonical,
        removed_guid,
        49_633,
        Position::new(21.0, 21.0, 0.0, 0.0),
        [0.0, 0.0, 0.0, 1.0],
        571,
        0,
    );
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_game_object_mut(in_world_guid)
        .unwrap()
        .world_mut()
        .object_mut()
        .add_to_world();
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .get_typed_game_object_mut(removed_guid)
        .unwrap()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &player_position, 800.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, in_world_guid);
}
#[test]
fn visible_gameobjects_canonical_create_uses_viewer_dynamic_flags_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let gameobject_guid = test_gameobject_guid(49_631, 49_631);

    session.set_player_game_master_like_cpp(true);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_lifecycle_gameobject_on_map(
        &canonical,
        gameobject_guid,
        49_631,
        Position::new(20.0, 20.0, 0.0, 0.0),
        [0.0, 0.0, 0.0, 1.0],
        571,
        0,
    );

    let visible = session
        .visible_gameobjects_from_canonical_map_like_cpp(571, &player_position, 800.0)
        .expect("canonical map");

    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].guid, gameobject_guid);
    assert_eq!(
        visible[0].dynamic_flags,
        wow_entities::GO_DYNFLAG_LO_ACTIVATE,
        "C++ ViewerDependentValue<DynamicFlagsTag> sets ACTIVATE for GM chest create"
    );
}
#[tokio::test]
async fn send_nearby_creatures_empty_map_source_without_world_db_clears_stale_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let stale_creature = test_creature_guid(44);

    session.set_map_manager(Arc::clone(&manager));
    session.client_visible_guids_like_cpp.insert(stale_creature);

    session
        .send_nearby_creatures(571, &player_position, 0)
        .await;

    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&stale_creature)
    );
    assert_eq!(session.last_visibility_pos, Some(player_position));
    assert!(
        send_rx.try_recv().is_err(),
        "empty map-owned creature source without world DB should not send creates"
    );
}
#[tokio::test]
async fn send_nearby_gameobjects_empty_canonical_source_is_authoritative_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let stale_gameobject = test_gameobject_guid(913, 45);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session
        .client_visible_guids_like_cpp
        .insert(stale_gameobject);

    session
        .send_nearby_gameobjects(571, &player_position, 0)
        .await;

    assert!(
        !session
            .client_visible_guids_like_cpp
            .contains(&stale_gameobject)
    );
    assert!(
        send_rx.try_recv().is_err(),
        "empty canonical gameobject source should not fall back to SQL or send creates"
    );
}
#[tokio::test]
async fn send_nearby_gameobjects_does_not_recreate_known_gameobjects_like_cpp() {
    // Regression for the world-entry client crash: send_nearby_gameobjects must NOT
    // re-CREATE gameobjects the client already knows. C++ `Player::UpdateVisibilityOf`
    // (Player.cpp) only builds a CREATE when `!HaveAtClient` (guid not in
    // `m_clientGUIDs`). A duplicate CREATE for a known GUID is invalid and makes the
    // Wrath client reset the connection. Calling this twice for the same visible set
    // must create exactly once.
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_position = Position::new(10.0, 10.0, 0.0, 0.0);
    let gameobject_guid = test_gameobject_guid(910, 30);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    canonical.lock().unwrap().create_world_map(571, 0);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        910,
        Position::new(20.0, 20.0, 0.0, 0.0),
        3,
    );
    session.record_represented_gameobject_display_model_like_cpp(
        gameobject_guid,
        7000,
        1.5,
        [0.0, 0.0, 0.0, 1.0],
    );

    // First call: gameobject unknown to the client -> exactly one CREATE is sent.
    session
        .send_nearby_gameobjects(571, &player_position, 0)
        .await;
    assert!(
        send_rx.try_recv().is_ok(),
        "first call should send a CREATE for the new gameobject"
    );
    assert!(
        send_rx.try_recv().is_err(),
        "first call should send exactly one packet"
    );
    assert!(
        session
            .client_visible_guids_like_cpp
            .contains(&gameobject_guid),
        "gameobject should be tracked as known after the first create"
    );

    // Second call: gameobject already known -> NO duplicate CREATE (the bug).
    session
        .send_nearby_gameobjects(571, &player_position, 0)
        .await;
    assert!(
        send_rx.try_recv().is_err(),
        "second call must not re-create an already-known gameobject (duplicate CREATE crashes the client)"
    );
}
#[test]
fn represented_can_see_spellclick_any_uses_canonical_creature_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(120);

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
            npc_entry: 700,
            spell_id: 900,
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 700,
        |spell| spell == 900,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        700,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible
    );
}
#[test]
fn represented_spellclick_accepts_vehicle_guid_as_creature_or_vehicle_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let vehicle_guid = test_vehicle_guid(231);

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
            npc_entry: 9006,
            spell_id: 912,
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9006,
        |spell| spell == 912,
    )));
    add_canonical_test_creature(
        &canonical,
        vehicle_guid,
        9006,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    assert!(vehicle_guid.is_vehicle());
    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(vehicle_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible,
        "C++ ObjectAccessor::GetCreatureOrPetOrVehicle accepts guid.IsCreatureOrVehicle()"
    );
    let plan = session.represented_handle_spell_click_plan_like_cpp(vehicle_guid);
    assert_eq!(plan.casts.len(), 1);
    assert_eq!(plan.casts[0].spell_id, 912);
    assert_eq!(
        plan.casts[0].caster,
        RepresentedSpellClickUnitRefLikeCpp::Clicker
    );
    assert_eq!(
        plan.casts[0].target,
        RepresentedSpellClickUnitRefLikeCpp::Clickee
    );
}
#[tokio::test]
async fn represented_spellclick_ignores_not_in_world_creature_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(228);

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
            npc_entry: 9003,
            spell_id: 909,
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9003,
        |spell| spell == 909,
    )));
    add_canonical_test_creature_on_map_with_world_state(
        &canonical,
        creature_guid,
        9003,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
        571,
        0,
        false,
    );

    assert_eq!(
        session.represented_can_see_spell_click_on_creature_like_cpp(creature_guid),
        RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden,
        "C++ HandleSpellClick returns silently when ObjectAccessor finds a unit that is not in world"
    );
    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    assert_eq!(plan, RepresentedSpellClickPlanLikeCpp::default());
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;
    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp::default()
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn npc_interaction_falls_back_to_legacy_creature_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let questgiver_guid = test_creature_guid(13);
    let manager = shared_map_manager();

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
    manager.write().unwrap().add_creature(
        571,
        0,
        0,
        0,
        crate::map_manager::WorldCreature::new(
            questgiver_guid,
            503,
            Position::new(14.0, 0.0, 0.0, 0.0),
            100,
            80,
            1,
            2,
            0.0,
            1,
            35,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
    );
    session.set_map_manager(manager);

    assert_eq!(
        session.represented_npc_can_interact_with_like_cpp(
            questgiver_guid,
            wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            0,
        ),
        Some(RepresentedCreatureAccessLikeCpp {
            entry: 503,
            position: Position::new(14.0, 0.0, 0.0, 0.0),
            npc_flags: wow_constants::unit::NPCFlags1::QUEST_GIVER.bits(),
            npc_flags2: 0,
            faction_template_id: 35,
            trainer_class: 0,
        })
    );
}
#[test]
fn creature_create_stats_uses_spawn_curmana_when_health_regen_disabled_like_cpp() {
    let (mut session, _, _) = make_session();
    let difficulty = Arc::new(wow_data::CreatureDifficultyStoreLikeCpp::from_records(
        [wow_data::CreatureDifficultyRecordLikeCpp {
            entry: 90_021,
            difficulty_id: 0,
            min_level: 5,
            max_level: 5,
            health_scaling_expansion: 0,
            health_modifier: 1.0,
            mana_modifier: 1.0,
            armor_modifier: 1.0,
            damage_modifier: 1.0,
            creature_difficulty_id: 0,
            type_flags: 0,
            type_flags2: 0,
            loot_id: 0,
            pickpocket_loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            static_flags: [0; 8],
        }],
        |_| 1.0,
    ));
    let base_stats = Arc::new(wow_data::CreatureBaseStatsStoreLikeCpp::from_records([
        (
            5,
            2,
            wow_data::CreatureBaseStatsRecordLikeCpp {
                base_health: [100, 100, 100],
                base_mana: 600,
                base_armor: 0,
                attack_power: 0,
                ranged_attack_power: 0,
                base_damage: [0.0; 3],
            },
        ),
        (
            5,
            3,
            wow_data::CreatureBaseStatsRecordLikeCpp {
                base_health: [100, 100, 100],
                base_mana: 600,
                base_armor: 0,
                attack_power: 0,
                ranged_attack_power: 0,
                base_damage: [0.0; 3],
            },
        ),
    ]));
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([
        wow_data::character_progression::ChrClassesEntry {
            id: 2,
            display_power: PowerType::Mana as u8,
            ..Default::default()
        },
        wow_data::character_progression::ChrClassesEntry {
            id: 3,
            display_power: PowerType::Focus as u8,
            ..Default::default()
        },
    ])));
    let power_types = Arc::new(PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 800,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Mana as i8,
            min_power: 0,
            max_base_power: 0,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace: 0.0,
            regen_combat: 0.0,
            flags: 0x0080,
        },
        wow_data::character_progression::PowerTypeEntry {
            id: 802,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Focus as i8,
            min_power: 0,
            max_base_power: 100,
            center_power: 0,
            default_power: 25,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace: 0.0,
            regen_combat: 0.0,
            flags: 0x0020 | 0x0080,
        },
    ]));
    let creature_spawn_catalogs = CreatureSpawnCatalogsLikeCpp {
        difficulty,
        base_stats,
        power_types,
        ..Default::default()
    };

    let full = session.creature_create_stats_with_catalogs_like_cpp(
        &creature_spawn_catalogs,
        90_021,
        5,
        2,
        0,
        true,
        42,
        77,
    );
    assert_eq!(
        full.power, 600,
        "C++ SetFullPower(POWER_MANA) uses max/base mana when _regenerateHealth is true"
    );

    let from_spawn = session.creature_create_stats_with_catalogs_like_cpp(
        &creature_spawn_catalogs,
        90_021,
        5,
        2,
        0,
        false,
        42,
        77,
    );
    assert_eq!(
        from_spawn.power, 77,
        "C++ SetSpawnHealth copies CreatureData::curmana when _regenerateHealth is false"
    );
    assert_eq!(from_spawn.base_mana, 600);

    let focus = session.creature_create_stats_with_catalogs_like_cpp(
        &creature_spawn_catalogs,
        90_021,
        5,
        3,
        0,
        false,
        42,
        77,
    );
    assert_eq!(focus.power_type, PowerType::Focus);
    assert_eq!(focus.max_power, 100);
    assert_eq!(
        focus.power, 25,
        "C++ SetSpawnHealth writes curmana through POWER_MANA and leaves Focus at DB2 DefaultPower"
    );
}
#[test]
fn creature_kill_reputation_mutates_and_sends_state_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let creature_guid = test_creature_guid(69201);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    let mut faction = FactionEntry::for_test_like_cpp(7, 5);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let creature_template_store = creature_template_lifecycle_store_for_test([9001]);
    let (onkill_store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
                creature_id: 9001,
                entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: 7,
                    rep_faction_2: 0,
                    reputation_max_cap_1: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_1: 250,
                    reputation_max_cap_2: 0,
                    rep_value_2: 0,
                    is_team_award_1: false,
                    is_team_award_2: false,
                    team_dependent: false,
                },
            }],
            &creature_template_store,
            &faction_store,
        );
    assert_eq!(report.loaded, 1);

    session.set_faction_store(Arc::new(faction_store));
    session.set_creature_onkill_reputation_store(Arc::new(onkill_store));

    session.reward_reputation_from_creature_kill_like_cpp(9001, creature_guid, 80, 1.0);

    let state = session
        .reputation_mgr_like_cpp()
        .get_state(5)
        .expect("faction state");
    assert_eq!(state.standing, 250);
    assert_eq!(
        session.represented_creature_kill_events_like_cpp(),
        &[
            RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                creature_guid,
                faction_id: 7,
                reputation: 250,
                spillover_only: false,
            }
        ]
    );

    let packet = drain_server_packet_bytes(&send_rx)
        .into_iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SetFactionStanding)
        })
        .expect("set faction standing packet");
    let mut reader = wow_packet::WorldPacket::from_bytes(&packet);
    reader.skip_opcode();
    assert_eq!(reader.read_float().unwrap(), 0.0);
    assert_eq!(reader.read_uint32().unwrap(), 1);
    assert_eq!(reader.read_int32().unwrap(), 5);
    assert_eq!(reader.read_int32().unwrap(), 250);
    assert!(!reader.read_bit().unwrap());
}
#[tokio::test]
async fn creature_kill_reputation_applies_group_rate_outside_dungeon_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let player_guid = ObjectGuid::create_player(1, 69_401);
    let other_guid = ObjectGuid::create_player(1, 69_402);
    let creature_guid = test_creature_guid(69_401);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    configure_two_player_group_for_reputation_test(&mut session, player_guid, other_guid);
    configure_single_creature_kill_reputation_for_test(&mut session);
    register_test_creature(&mut session, manager, creature_guid, 50);

    session
        .apply_damage(None, creature_guid, 100)
        .await
        .unwrap();

    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .unwrap()
            .standing,
        125
    );
    assert_eq!(
        session
            .represented_creature_kill_events_like_cpp()
            .iter()
            .filter(|event| matches!(
                event,
                RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                    creature_guid: guid,
                    faction_id: 7,
                    reputation: 125,
                    spillover_only: false,
                } if *guid == creature_guid
            ))
            .count(),
        1
    );
}
#[tokio::test]
async fn creature_kill_reputation_forces_full_rate_in_dungeon_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let manager = shared_map_manager();
    let player_guid = ObjectGuid::create_player(1, 69_403);
    let other_guid = ObjectGuid::create_player(1, 69_404);
    let creature_guid = test_creature_guid(69_402);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
    session.set_player_map_position_like_cpp(0, Position::new(10.0, 10.0, 0.0, 0.0));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    configure_two_player_group_for_reputation_test(&mut session, player_guid, other_guid);
    configure_single_creature_kill_reputation_for_test(&mut session);
    register_test_creature(&mut session, manager, creature_guid, 50);

    session
        .apply_damage(None, creature_guid, 100)
        .await
        .unwrap();

    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .unwrap()
            .standing,
        250
    );
}
