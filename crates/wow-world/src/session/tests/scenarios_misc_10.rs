//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn stop_player_attack_canonical_no_victim_ignores_stale_session_target_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 80);
    let stale_victim = test_creature_guid(18_031);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
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
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Idle".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.combat_target = Some(stale_victim);
    session.in_combat = true;

    assert_eq!(session.stop_player_attack_like_cpp(), None);
    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));
}
#[test]
fn player_attack_from_vehicle_seat_requires_can_attack_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 61);
    let blocked_victim = test_creature_guid(18_011);
    let allowed_victim = test_creature_guid(18_012);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Passenger".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));

    assert!(session.set_player_vehicle_seat_state_like_cpp(Some(0), None));
    session.start_player_attack_like_cpp(blocked_victim);

    {
        let guard = canonical.lock().unwrap();
        let player_entity = guard
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap();
        assert_eq!(player_entity.unit().attacking(), None);
        assert_eq!(player_entity.unit().data().target, ObjectGuid::EMPTY);
    }
    assert_eq!(session.resolved_combat_target_like_cpp(), Some(None));
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(false));

    assert!(session.set_player_vehicle_seat_state_like_cpp(
        Some(wow_data::VEHICLE_SEAT_FLAG_CAN_ATTACK),
        None,
    ));
    session.start_player_attack_like_cpp(allowed_victim);

    let guard = canonical.lock().unwrap();
    let player_entity = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(player)
        .unwrap();
    assert_eq!(player_entity.unit().attacking(), Some(allowed_victim));
    assert_eq!(player_entity.unit().data().target, allowed_victim);
    drop(guard);
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(allowed_victim))
    );
}
#[test]
fn kick_sets_disconnecting() {
    let (mut session, _, _) = make_session();
    session.kick("test");
    assert!(session.is_disconnecting());
}
#[tokio::test]
async fn disconnected_channel_sets_disconnecting() {
    let (mut session, pkt_tx, _) = make_session();
    drop(pkt_tx); // Close the channel

    session.update(100).await;
    assert!(session.is_disconnecting());
}
#[test]
fn send_packet_works() {
    let (session, _, send_rx) = make_session();

    let pong = wow_packet::packets::auth::Pong { serial: 42 };
    session.send_packet(&pong);

    let data = send_rx.try_recv().unwrap();
    assert_eq!(data.len(), 6); // opcode(2) + serial(4)
}
#[test]
fn send_update_world_state_like_cpp_hidden_sets_final_bit_byte() {
    let (session, _, send_rx) = make_session();
    let variable_id = 0x5566_7788;
    let value = 42;

    session.send_update_world_state_like_cpp(variable_id, value, true);

    let data = send_rx.try_recv().unwrap();
    assert_eq!(data.len(), 11);
    assert_eq!(
        u16::from_le_bytes([data[0], data[1]]),
        ServerOpcodes::UpdateWorldState as u16
    );
    assert_eq!(&data[2..6], &variable_id.to_le_bytes());
    assert_eq!(&data[6..10], &value.to_le_bytes());
    assert_eq!(data[10], 0x80);
    assert_eq!(send_rx.try_recv(), Err(flume::TryRecvError::Empty));
}
#[test]
fn send_update_world_state_like_cpp_uses_only_this_session_channel() {
    let (session, _, send_rx) = make_session();
    let (_other_session, _, other_send_rx) = make_session();

    session.send_update_world_state_like_cpp(0x0102_0304, 7, false);

    assert_eq!(drain_server_packet_bytes(&send_rx).len(), 1);
    assert_eq!(other_send_rx.try_recv(), Err(flume::TryRecvError::Empty));
}
#[test]
fn toy_clear_fanfare_clears_known_toy_only_like_cpp() {
    let (mut session, _, _) = make_session();
    session.load_represented_account_toys_like_cpp([(30_000, true, true), (30_001, false, true)]);

    assert!(session.toy_clear_fanfare_like_cpp(30_000));
    assert!(!session.toy_clear_fanfare_like_cpp(40_000));

    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, true, false), (30_001, false, true)]
    );
}
#[test]
fn toy_set_favorite_toggles_known_toy_only_like_cpp() {
    let (mut session, _, _) = make_session();
    session.load_represented_account_toys_like_cpp([(30_000, false, true)]);

    assert!(session.toy_set_favorite_like_cpp(30_000, true));
    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, true, true)]
    );

    assert!(session.toy_set_favorite_like_cpp(30_000, false));
    assert!(!session.toy_set_favorite_like_cpp(40_000, true));
    assert_eq!(
        session.account_toy_rows_like_cpp(),
        vec![(30_000, false, true)]
    );
}
#[test]
fn player_is_possessing_requires_possessed_charmed_unit_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 31_000);
    let controlled_guid = test_creature_guid(31_001);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(10.0, 10.0, 0.0, 0.0),
        571,
        0,
    );
    add_canonical_test_creature(
        &canonical,
        controlled_guid,
        9001,
        Position::new(11.0, 10.0, 0.0, 0.0),
        0,
    );

    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(571, 0).unwrap().map_mut();
        map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .apply_charm_as_controller(controlled_guid, true);
        assert!(
            map.get_typed_creature_mut(controlled_guid)
                .unwrap()
                .unit_mut()
                .subsystems_mut()
                .control
                .apply_charmed_by(player_guid, CharmType::Charm, true, None, false)
        );
    }

    assert!(!session.player_is_possessing_like_cpp());

    {
        let mut guard = canonical.lock().unwrap();
        guard
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(controlled_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .charm_type = Some(CharmType::Possess);
    }

    assert!(session.player_is_possessing_like_cpp());
}
#[test]
fn appearance_favorite_state_transitions_match_collection_mgr_like_cpp() {
    let (mut session, _, send_rx) = make_session();

    assert!(session.set_appearance_is_favorite_like_cpp(65, true));
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(65),
        Some(FavoriteAppearanceStateLikeCpp::New)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(!session.set_appearance_is_favorite_like_cpp(65, true));
    assert!(send_rx.try_recv().is_err());

    assert!(session.set_appearance_is_favorite_like_cpp(65, false));
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(65),
        None
    );
    assert!(send_rx.try_recv().is_err());
    assert!(!session.set_appearance_is_favorite_like_cpp(65, false));
    assert!(send_rx.try_recv().is_err());

    session
        .represented_favorite_item_appearances_like_cpp
        .insert(96, FavoriteAppearanceStateLikeCpp::Unchanged);
    assert!(session.set_appearance_is_favorite_like_cpp(96, false));
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(96),
        Some(FavoriteAppearanceStateLikeCpp::Removed)
    );
    assert!(send_rx.try_recv().is_err());
    assert!(session.set_appearance_is_favorite_like_cpp(96, true));
    assert_eq!(
        session.represented_favorite_item_appearance_state_like_cpp(96),
        Some(FavoriteAppearanceStateLikeCpp::Unchanged)
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn total_stat_percentage_uses_each_active_effect_amount_and_selector_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 16);
    session.set_player_guid(Some(player_guid));

    let mut spell_store = SpellStore::new();
    spell_store.insert(
        20_599,
        SpellInfo {
            spell_id: 20_599,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                    effect_base_points: 9,
                    effect_die_sides: 1,
                    effect_misc_value_1: 0,
                    effect_misc_value_2: 1 << 0,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                    effect_base_points: 19,
                    effect_die_sides: 1,
                    effect_misc_value_1: 1,
                    effect_misc_value_2: 1 << 1,
                    ..Default::default()
                },
            ],
            ..test_spell_info_like_cpp(20_599)
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .apply_aura_with_effect_mask_without_update_like_cpp(
            20_599,
            player_guid,
            30_000,
            AFLAG_NOCASTER_LIKE_CPP,
            0b11,
        )
        .expect("apply multi-effect total-stat aura");

    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.1, 1.2, 1.0, 1.0, 1.0]
    );
    assert_eq!(
        session.represented_total_stat_buff_multipliers_like_cpp(),
        [1.1, 1.2, 1.0, 1.0, 1.0]
    );
    let aura = session
        .visible_auras
        .values()
        .find(|aura| aura.spell_id == 20_599)
        .expect("multi-effect aura");
    assert_eq!(
        aura.represented_effect_amounts,
        vec![
            RepresentedAuraEffectAmountLikeCpp {
                effect_index: 0,
                amount: 10,
            },
            RepresentedAuraEffectAmountLikeCpp {
                effect_index: 1,
                amount: 20,
            },
        ]
    );
}
#[test]
fn player_registry_relation_snapshot_syncs_from_session_and_canonical_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_registry = Arc::new(PlayerRegistry::default());
    let player_guid = ObjectGuid::create_player(1, 606);

    session.set_player_guid(Some(player_guid));
    session.player_name = Some("RelationSnapshot".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    session.set_player_faction_template_like_cpp(1);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .gameplay_state_mut()
            .reputations
            .push(wow_entities::PlayerReputationRecord {
                faction_id: 72,
                standing: -6000,
                flags: wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP,
                ..Default::default()
            });
        player.set_forced_reputation_rank_like_cpp(87, true);
        player.gameplay_state_mut().forced_reputation_ranks = vec![(
            87,
            wow_data::reputation::ReputationRankLikeCpp::Hostile.as_u8(),
        )];
        player
            .unit_mut()
            .set_unit_flags2_like_cpp(UnitFlags2::IGNORE_REPUTATION);
        player.unit_mut().set_combat_reach(1.25);
        player.set_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
    });

    session.register_in_player_registry();

    let snapshot = player_registry
        .legacy_aggro_candidates()
        .into_iter()
        .find(|candidate| candidate.player_guid == player_guid)
        .expect("canonical aggro snapshot");
    assert_eq!(snapshot.faction_template_id, 1);
    assert_eq!(
        snapshot.forced_reputation_ranks,
        vec![(87, wow_data::reputation::ReputationRankLikeCpp::Hostile)]
    );
    assert_eq!(snapshot.combat_reach, 1.25);
}
#[test]
fn player_registry_targetability_snapshot_syncs_from_session_and_canonical_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_registry = Arc::new(PlayerRegistry::default());
    let player_guid = ObjectGuid::create_player(1, 605);

    session.set_player_guid(Some(player_guid));
    session.player_name = Some("TargetabilitySnapshot".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    session
        .player_unit_flags_like_cpp
        .insert(UnitFlags::PLAYER_CONTROLLED | UnitFlags::ON_TAXI);
    session.set_player_game_master_like_cpp(true);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&player_registry));
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().add_unit_state(UnitState::DIED.bits());
    });

    session.register_in_player_registry();

    let snapshot = player_registry
        .legacy_aggro_candidates()
        .into_iter()
        .find(|candidate| candidate.player_guid == player_guid)
        .expect("canonical targetability snapshot");
    assert_eq!(
        snapshot.unit_flags,
        (UnitFlags::PLAYER_CONTROLLED | UnitFlags::ON_TAXI).bits()
    );
    assert_eq!(snapshot.unit_state, UnitState::DIED.bits());
    assert!(snapshot.is_game_master);
}
#[test]
fn toggle_pvp_sets_in_pvp_flag_and_enables_pvp_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
        })
        .unwrap();
    session.player_pvp_enabled_like_cpp = false;
    session.player_pvp_end_timer_like_cpp = Some(123);

    session.apply_toggle_pvp_like_cpp();

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP));
    assert!(!player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert!(session.player_in_pvp_flag_like_cpp);
    assert!(session.player_pvp_enabled_like_cpp);
    assert_eq!(session.player_pvp_end_timer_like_cpp, None);
}
#[test]
fn toggle_pvp_removes_in_pvp_and_starts_timer_when_not_hostile_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    session.player_in_pvp_flag_like_cpp = true;
    session.player_pvp_enabled_like_cpp = true;
    session.player_pvp_hostile_like_cpp = false;

    session.apply_toggle_pvp_like_cpp();

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(!player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP));
    assert!(player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert!(!session.player_in_pvp_flag_like_cpp);
    assert_eq!(session.player_pvp_enabled_like_cpp, true);
    assert!(session.player_pvp_end_timer_like_cpp.is_some());
}
#[test]
fn toggle_pvp_does_not_toggle_off_with_war_mode_local_active_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    session.player_in_pvp_flag_like_cpp = true;
    session.player_pvp_enabled_like_cpp = true;
    session.set_active_player_local_flags_like_cpp(PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP);

    session.apply_toggle_pvp_like_cpp();

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP));
    assert!(!player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert_eq!(session.player_pvp_end_timer_like_cpp, None);
}
#[test]
fn set_pvp_enable_sets_in_pvp_flag_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
        })
        .unwrap();
    session.player_pvp_enabled_like_cpp = false;
    session.player_pvp_end_timer_like_cpp = Some(123);

    session.apply_set_pvp_like_cpp(true);

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP));
    assert!(!player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert!(session.player_in_pvp_flag_like_cpp);
    assert!(session.player_pvp_enabled_like_cpp);
    assert_eq!(session.player_pvp_end_timer_like_cpp, None);
}
#[test]
fn set_pvp_disable_starts_timer_without_toggling_back_on_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    session.player_in_pvp_flag_like_cpp = true;
    session.player_pvp_enabled_like_cpp = true;
    session.player_pvp_hostile_like_cpp = false;

    session.apply_set_pvp_like_cpp(false);

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(!player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP));
    assert!(player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert!(!session.player_in_pvp_flag_like_cpp);
    assert_eq!(session.player_pvp_enabled_like_cpp, true);
    assert!(session.player_pvp_end_timer_like_cpp.is_some());
}
#[test]
fn update_pvp_flag_expires_timer_after_five_minutes_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    session.player_pvp_enabled_like_cpp = true;
    session.player_pvp_hostile_like_cpp = false;
    session.player_pvp_end_timer_like_cpp = Some(1_000);

    session.update_pvp_flag_like_cpp(1_300);

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(!player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(!player.unit().is_pvp_like_cpp());
    assert!(!session.player_pvp_enabled_like_cpp);
    assert_eq!(session.player_pvp_end_timer_like_cpp, None);
}
#[test]
fn update_pvp_flag_keeps_timer_before_five_minutes_like_cpp() {
    let (mut session, canonical, guid) = session_with_canonical_player_for_away_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    session.player_pvp_enabled_like_cpp = true;
    session.player_pvp_hostile_like_cpp = false;
    session.player_pvp_end_timer_like_cpp = Some(1_000);

    session.update_pvp_flag_like_cpp(1_299);

    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(player.unit().is_pvp_like_cpp());
    assert!(session.player_pvp_enabled_like_cpp);
    assert_eq!(session.player_pvp_end_timer_like_cpp, Some(1_000));
}
#[tokio::test]
async fn logged_in_update_consumes_expired_pvp_timer_like_cpp() {
    let (mut session, _pkt_tx, canonical, guid) =
        session_with_canonical_player_for_away_like_cpp_with_packet_tx();
    session.set_state(SessionState::LoggedIn);
    session.socket_timeout_deadline_like_cpp = Instant::now() + Duration::from_secs(60);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
        })
        .unwrap();
    let now = wow_entities::game_time_secs_like_cpp();
    session.player_pvp_enabled_like_cpp = true;
    session.player_pvp_hostile_like_cpp = false;
    session.player_pvp_end_timer_like_cpp = Some(now - 301);

    assert_eq!(session.state(), SessionState::LoggedIn);
    let _ = session.update(50).await;

    assert_eq!(session.player_pvp_end_timer_like_cpp, None);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_player(guid)
        .unwrap();
    assert!(!player.has_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP));
    assert!(!player.unit().is_pvp_like_cpp());
}
#[test]
fn far_sight_disable_sets_represented_seer_back_to_self_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 4250);
    let target_guid = test_creature_guid(4251);

    session.set_player_guid(Some(player_guid));
    session.represented_seer_guid_like_cpp = Some(target_guid);

    session.apply_far_sight_like_cpp(false);

    assert_eq!(session.represented_seer_guid_like_cpp(), Some(player_guid));
}
#[tokio::test]
async fn cross_socket_fence_timeout_stops_later_channel_publication_like_cpp() {
    let (mut session, _, _instance_rx) = make_session();
    let (realm_tx, _realm_rx) = flume::unbounded();
    session.install_realm_send_channel_for_test(realm_tx);
    session.set_send_write_fence_like_cpp(SocketWriteFenceLikeCpp::default());
    session.install_realm_send_write_fence_for_test(SocketWriteFenceLikeCpp::default());

    assert!(
        !session
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await,
        "an unacknowledged instance fence cannot authorize realm publication"
    );
    assert!(
        !session
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await,
        "an unacknowledged realm fence cannot authorize instance publication"
    );
    assert_ne!(session.state(), SessionState::Disconnecting);
}
#[test]
fn state_transitions() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.state(), SessionState::Authed);

    session.set_state(SessionState::LoggedIn);
    assert_eq!(session.state(), SessionState::LoggedIn);

    session.set_state(SessionState::Transfer);
    assert_eq!(session.state(), SessionState::Transfer);
}
#[test]
fn legit_characters_management() {
    let (mut session, _, _) = make_session();

    let guid1 = ObjectGuid::create_player(1, 1);
    let guid2 = ObjectGuid::create_player(1, 2);
    let guid3 = ObjectGuid::create_player(1, 3);

    session.set_legit_characters(vec![guid1, guid2, guid3]);
    assert!(session.is_legit_character(&guid1));
    assert!(session.is_legit_character(&guid2));
    assert!(!session.is_legit_character(&ObjectGuid::create_player(1, 99)));

    session.remove_legit_character(&guid2);
    assert!(!session.is_legit_character(&guid2));
    assert!(session.is_legit_character(&guid1));
}
#[test]
fn realm_id_and_virtual_address_defaults() {
    let (mut session, _, _) = make_session();

    assert_eq!(session.realm_id(), 1);
    assert_eq!(session.virtual_realm_address(), 0x0101_0001);

    session.set_realm_id(5);
    assert_eq!(session.realm_id(), 5);
    assert_eq!(session.virtual_realm_address(), 0x0101_0005);
}
#[test]
fn realm_query_response_uses_realm_list_names_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_realm_handle_like_cpp(5, 6, 9);
    session.set_realm_names_like_cpp([
        (0x0506_0009, "Ice Crown".to_string(), "IceCrown".to_string()),
        (
            0x0708_000A,
            "Remote Realm".to_string(),
            "RemoteRealm".to_string(),
        ),
    ]);

    let local = session.realm_query_response_like_cpp(0x0506_0009);
    assert_eq!(local.lookup_state, 0);
    assert_eq!(local.realm_name_actual, "Ice Crown");
    assert_eq!(local.realm_name_normalized, "IceCrown");
    assert!(local.is_local);

    let remote = session.realm_query_response_like_cpp(0x0708_000A);
    assert_eq!(remote.lookup_state, 0);
    assert_eq!(remote.realm_name_actual, "Remote Realm");
    assert_eq!(remote.realm_name_normalized, "RemoteRealm");
    assert!(!remote.is_local);

    let missing = session.realm_query_response_like_cpp(0x0909_000B);
    assert_eq!(missing.lookup_state, 1);
    assert!(missing.realm_name_actual.is_empty());
    assert!(missing.realm_name_normalized.is_empty());
    assert!(!missing.is_local);
}
#[test]
fn represented_faction_reaction_static_branch_uses_forced_rank_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 72, 0, 0, 0),
            faction_template_entry(2, 930, 0, 0, 0),
        ]),
    ));
    session
        .reputation_mgr_like_cpp_mut()
        .apply_force_reaction_like_cpp(
            72,
            wow_data::reputation::ReputationRankLikeCpp::Hostile,
            true,
        );

    assert_eq!(
        session.represented_faction_reaction_to_like_cpp(RepresentedFactionReactionInputLikeCpp {
            source_faction_template_id: 1,
            target_faction_template_id: 2,
            target_has_player_owner: true,
            target_player_owner_is_current_session: true,
            target_player_contested_pvp: false,
            target_is_unit: true,
            target_ignores_reputation: false,
        },),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );
}
#[test]
fn represented_get_reaction_wrapper_top_branches_match_cpp() {
    let (session, _pkt_tx, _send_rx) = make_session();
    let mut input = represented_get_reaction_input_like_cpp();

    input.same_object = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );

    input.same_object = false;
    input.attackable_by_summoner = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Neutral
    );

    input.attackable_by_summoner = false;
    input.same_charmer_or_owner_or_self = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );
}
#[test]
fn represented_get_reaction_target_owner_forced_rank_branch_matches_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 72, 0, 0, 0),
            faction_template_entry(2, 930, 0, 0, 0),
        ]),
    ));

    let mut input = represented_get_reaction_input_like_cpp();
    input.self_has_player_owner = false;
    input.target_player_owner_is_current_session = false;
    input.self_unit_player_controlled = false;
    input.target_owner_forced_rank_for_self =
        Some(wow_data::reputation::ReputationRankLikeCpp::Revered);

    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Revered
    );

    input.self_faction_template_id = 99;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Neutral
    );
}
