//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_attack_accepts_typed_player_victim_with_pvp_flag_snapshot_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 49);
    let victim = ObjectGuid::create_player(1, 50);

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
        attacker,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    let mut victim_player = Player::new(Some(8), false);
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(victim);
    victim_player
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    victim_player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(11.0, 20.0, 30.0, 0.0));
    victim_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    victim_player
        .unit_mut()
        .set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(victim_player).unwrap())
        .unwrap();

    session.start_player_attack_like_cpp(victim);
    {
        let guard = canonical.lock().unwrap();
        let map = guard.find_map(571, 0).unwrap().map();
        let attacker_entity = map.get_typed_player(attacker).unwrap();
        let victim_entity = map.get_typed_player(victim).unwrap();
        assert_eq!(attacker_entity.unit().attacking(), Some(victim));
        assert_eq!(attacker_entity.unit().data().target, victim);
        assert!(victim_entity.unit().has_attacker_like_cpp(attacker));
    }
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
#[test]
fn summon_private_object_owner_derives_from_properties_like_cpp() {
    let (mut session, _, _) = make_session();
    let caster = ObjectGuid::create_player(1, 66);
    let inherited_private_owner = ObjectGuid::create_player(1, 67);

    let mut properties = SummonPropertiesEntry {
        id: 1,
        control: 0,
        faction: 0,
        title: 0,
        slot: 0,
        flags: [0, 0],
    };
    assert_eq!(
        session.summon_private_object_owner_like_cpp(caster, ObjectGuid::EMPTY, &properties),
        ObjectGuid::EMPTY
    );

    properties.flags[0] = SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP as i32;
    assert_eq!(
        session.summon_private_object_owner_like_cpp(caster, ObjectGuid::EMPTY, &properties),
        caster
    );
    assert_eq!(
        session.summon_private_object_owner_like_cpp(caster, inherited_private_owner, &properties),
        inherited_private_owner
    );

    properties.flags[0] = SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP as i32;
    assert_eq!(
        session.summon_private_object_owner_like_cpp(caster, ObjectGuid::EMPTY, &properties),
        caster
    );
    session.group_guid = Some(77);
    assert_eq!(
        session.summon_private_object_owner_like_cpp(caster, ObjectGuid::EMPTY, &properties),
        ObjectGuid::create_group(77)
    );
}
#[test]
fn player_attack_can_always_see_unit_being_moved_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let attacker = ObjectGuid::create_player(1, 69);
    let controlled = test_creature_guid(670);

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
        attacker,
        "Hunter".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        3,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.apply_move_init_active_mover_complete_like_cpp(0);
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(controlled, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_canonical_creature_by_guid_like_cpp(controlled, |creature| {
            creature.unit_mut().set_invisibility_like_cpp(0, 100);
        })
        .unwrap();

    assert_eq!(
        session.start_player_attack_like_cpp(controlled),
        PlayerAttackStartLikeCppResult::Rejected
    );

    session.set_player_moved_unit_guid_like_cpp(controlled);
    assert_eq!(
        session.start_player_attack_like_cpp(controlled),
        PlayerAttackStartLikeCppResult::Accepted {
            send_attack_start: true
        }
    );
}
#[test]
fn offline_rested_xp_tavern_accrues_more_than_wilderness_like_cpp() {
    let (mut wilderness, _, _) = make_session();
    wilderness.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    wilderness.set_player_next_level_xp_like_cpp(72_000);
    wilderness.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let (mut tavern, _, _) = make_session();
    tavern.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    tavern.set_player_next_level_xp_like_cpp(72_000);
    tavern.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let wilderness_extra = wilderness.apply_offline_xp_rest_bonus_like_cpp(1_000, 4_600, false);
    let tavern_extra = tavern.apply_offline_xp_rest_bonus_like_cpp(1_000, 4_600, true);

    assert!((wilderness_extra - 111.6).abs() < 0.01);
    assert!((tavern_extra - 450.0).abs() < 0.01);
    assert!(tavern_extra > wilderness_extra);
    assert!(
        tavern.represented_xp_rest_bonus_like_cpp()
            > wilderness.represented_xp_rest_bonus_like_cpp()
    );
    assert_eq!(
        tavern.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
}
#[test]
fn rested_xp_uses_configured_rest_rates_like_cpp() {
    let policy = PlayerRestRatePolicyLikeCpp {
        offline_wilderness: 2.0,
        offline_tavern_or_city: 3.0,
        ingame: 4.0,
    };
    let (mut wilderness, _, _) = make_session();
    wilderness.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    wilderness.set_player_next_level_xp_like_cpp(72_000);
    wilderness.set_max_player_level_config_like_cpp(80);
    wilderness.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let (mut tavern, _, _) = make_session();
    tavern.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    tavern.set_player_next_level_xp_like_cpp(72_000);
    tavern.set_max_player_level_config_like_cpp(80);
    tavern.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);

    let wilderness_extra =
        wilderness.apply_offline_xp_rest_bonus_with_policy_like_cpp(&policy, 1_000, 4_600, false);
    let tavern_extra =
        tavern.apply_offline_xp_rest_bonus_with_policy_like_cpp(&policy, 1_000, 4_600, true);

    assert!((wilderness_extra - 223.2).abs() < 0.01);
    assert!((tavern_extra - 1_350.0).abs() < 0.01);

    let (mut online, _, _) = make_session();
    online.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    online.set_player_next_level_xp_like_cpp(72_000);
    online.set_max_player_level_config_like_cpp(80);
    online.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);
    assert!(online.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    online.represented_rest_time_secs_like_cpp = 1_000;

    let (online_extra, _) =
        online.update_represented_online_xp_rest_bonus_with_policy_like_cpp(&policy, 1_010);

    assert!((online_extra - 5.0).abs() < 0.01);
}
#[test]
fn negative_rest_rate_is_preserved_and_clamped_by_rest_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.set_rested_xp_config_like_cpp(80, -1.0, 1.0, 1.0);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 100.0);

    let extra = session.apply_offline_xp_rest_bonus_like_cpp(1_000, 4_600, false);

    assert!(extra < 0.0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}
#[test]
fn offline_rested_xp_caps_at_cpp_next_level_threshold_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 700.0);

    let extra = session.apply_offline_xp_rest_bonus_like_cpp(1, 1_000_000, true);

    assert!(extra > 0.0);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 750.0);
    assert_eq!(session.represented_xp_rest_threshold_like_cpp(), 750);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
}
#[test]
fn rested_xp_uses_configured_max_player_level_like_cpp() {
    let (mut session, _, _) = make_session();
    let victim = test_creature_guid(88);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 70, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_rested_xp_config_like_cpp(70, 1.0, 1.0, 1.0);
    install_tapped_xp_victim_like_cpp(&mut session, victim);

    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 100.0);

    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 100.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
    assert!(!session.give_xp_runtime_like_cpp(50, victim, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 0);
}
#[test]
fn give_xp_uses_active_expansion_max_level_like_cpp() {
    for (expansion, max_level) in [(0, 60), (1, 70)] {
        let (mut session, _, send_rx) = make_session();
        session.expansion = expansion;
        session.set_loaded_player_identity_like_cpp(1, 1, 8, max_level, 0);
        session.set_player_next_level_xp_like_cpp(1_000);
        session.set_rested_xp_config_like_cpp(80, 1.0, 1.0, 1.0);

        assert!(!session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
        assert_eq!(session.player_xp_like_cpp(), 0);
        assert!(drain_server_packet_bytes(&send_rx).is_empty());
    }
}
#[test]
fn active_player_max_level_combines_expansion_and_config_like_cpp() {
    for (expansion, configured, expected) in [(0, 80, 60), (1, 80, 70), (2, 70, 70), (2, 85, 85)] {
        let (mut session, _, _) = make_session();
        session.expansion = expansion;
        session.set_rested_xp_config_like_cpp(configured, 1.0, 1.0, 1.0);
        assert_eq!(session.player_active_max_level_like_cpp(), expected);
    }
}
#[test]
fn scaling_player_level_delta_uses_half_xp_and_compile_time_max_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_rested_xp_config_like_cpp(60, 1.0, 1.0, 1.0);
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 79, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_player_xp_like_cpp(499);
    assert_eq!(session.player_scaling_level_delta_like_cpp(), -1);

    session.set_player_xp_like_cpp(500);
    assert_eq!(session.player_scaling_level_delta_like_cpp(), 0);

    session.set_player_level_like_cpp(80);
    session.set_player_xp_like_cpp(0);
    assert_eq!(session.player_scaling_level_delta_like_cpp(), 0);
}
#[test]
fn give_xp_allows_custom_levels_for_current_expansion_like_cpp() {
    let (mut session, _, _) = make_session();
    session.expansion = 2;
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 80, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.set_rested_xp_config_like_cpp(85, 1.0, 1.0, 1.0);

    assert!(session.give_xp_runtime_like_cpp(50, ObjectGuid::EMPTY, 1.0));
    assert_eq!(session.player_xp_like_cpp(), 50);
}
#[test]
fn offline_rested_xp_invalid_or_max_level_cases_do_not_accrue_like_cpp() {
    let (mut no_next_level_xp, _, _) = make_session();
    no_next_level_xp.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    no_next_level_xp.set_player_next_level_xp_like_cpp(0);
    no_next_level_xp.load_represented_xp_rest_bonus_like_cpp(99, f32::NAN);

    let no_next_extra = no_next_level_xp.apply_offline_xp_rest_bonus_like_cpp(10, 3_610, true);

    assert_eq!(no_next_extra, 0.0);
    assert_eq!(no_next_level_xp.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        no_next_level_xp.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );

    let (mut max_level, _, _) = make_session();
    max_level.set_loaded_player_identity_like_cpp(1, 1, 8, 80, 0);
    max_level.set_player_next_level_xp_like_cpp(72_000);
    max_level.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RESTED_LIKE_CPP, 100.0);

    let max_level_extra = max_level.apply_offline_xp_rest_bonus_like_cpp(10, 3_610, true);

    assert_eq!(max_level_extra, 0.0);
    assert_eq!(max_level.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        max_level.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
}
#[test]
fn tavern_area_trigger_sets_and_clears_represented_resting_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, true));
    assert!(session.represented_is_resting_like_cpp());

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, false));
    assert!(!session.represented_is_resting_like_cpp());
    assert!(!session.handle_represented_tavern_area_trigger_like_cpp(77, true));
}
#[test]
fn ffa_realm_tavern_enter_removes_and_leave_restores_ffa_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 0xFFA4);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.ensure_login_player_controller_like_cpp(
        guid,
        "FfaTavern".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.set_ffa_pvp_realm_like_cpp(true);
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_state(SessionState::LoggedIn);
    assert!(
        session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP))
    );
    let _ = drain_server_packet_bytes(&send_rx);

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, true));
    assert!(session.represented_is_resting_like_cpp());
    assert!(
        !session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP))
    );
    assert_eq!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count(),
        2,
        "entering sends PlayerFlags::RESTING and UnitData::PvpFlags deltas"
    );

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, false));
    assert!(!session.represented_is_resting_like_cpp());
    assert!(
        session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP))
    );
    assert_eq!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count(),
        2,
        "leaving sends PlayerFlags::RESTING and UnitData::PvpFlags deltas"
    );
}
#[test]
fn ffa_realm_tavern_leave_restores_ffa_while_city_rest_remains_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 0xFFA6);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.ensure_login_player_controller_like_cpp(
        guid,
        "FfaCityInn".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.set_ffa_pvp_realm_like_cpp(true);
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_state(SessionState::LoggedIn);
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    let _ = drain_server_packet_bytes(&send_rx);

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, true));
    assert!(session.represented_is_resting_like_cpp());
    assert!(
        !session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP))
    );
    assert_eq!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count(),
        1,
        "overlapping city rest means only the FFA delta is visible on enter"
    );

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, false));
    assert!(
        session.represented_is_resting_like_cpp(),
        "C++ RestMgr keeps PLAYER_FLAGS_RESTING while the city bit remains"
    );
    assert!(
        session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP)),
        "C++ tavern handling restores FFA from Entered=false without consulting RestMgr"
    );
    assert_eq!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .filter(|packet| {
                WorldPacket::from_bytes(packet).server_opcode() == Some(ServerOpcodes::UpdateObject)
            })
            .count(),
        1,
        "overlapping city rest means only the FFA delta is visible on leave"
    );
}
#[test]
fn normal_realm_tavern_does_not_toggle_ffa_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xFFA5);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.ensure_login_player_controller_like_cpp(
        guid,
        "NormalTavern".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));

    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, true));
    assert!(session.handle_represented_tavern_area_trigger_like_cpp(42, false));
    assert!(
        !session
            .canonical_player_pvp_flags_like_cpp(guid)
            .is_some_and(|flags| flags.contains(UnitPvpFlags::FFA_PVP))
    );
}
#[test]
fn zero_integer_rest_award_normalizes_raf_state_without_touching_rest_flags_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 0xE1C6);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.ensure_login_player_controller_like_cpp(
        guid,
        "RestStateVsLocation".to_string(),
        Position::new(1.0, 2.0, 3.0, 0.0),
        1,
        1,
        8,
        10,
        0,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 1, 0);
    session.set_player_next_level_xp_like_cpp(1_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_RAF_LINKED_LIKE_CPP, 0.0);
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_TAVERN_LIKE_CPP, 42));
    let rest_time = session.represented_rest_time_secs_like_cpp;

    let (award, nested_mask) =
        session.take_represented_xp_rest_bonus_for_gain_like_cpp(50, test_creature_guid(0xE1C6));

    assert_eq!(award, 0);
    assert_eq!(nested_mask, 0x07);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_NORMAL_LIKE_CPP
    );
    assert_eq!(
        session.represented_rest_flag_mask_like_cpp,
        REST_FLAG_IN_TAVERN_LIKE_CPP
    );
    assert_eq!(session.represented_rest_time_secs_like_cpp, rest_time);
    assert_eq!(session.represented_inn_area_trigger_id_like_cpp, 42);
    assert!(session.represented_is_resting_like_cpp());
    assert!(
        session
            .canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_RESTING_LIKE_CPP)
            .unwrap_or(false),
        "RestInfo state and physical PLAYER_FLAGS_RESTING are independent in C++"
    );
}
#[test]
fn online_rest_update_adds_rested_xp_after_ten_seconds_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    session.represented_rest_time_secs_like_cpp = 1_000;

    assert_eq!(
        session.update_represented_online_xp_rest_bonus_like_cpp(1_009),
        (0.0, 0)
    );
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);

    let (extra, nested_mask) = session.update_represented_online_xp_rest_bonus_like_cpp(1_010);

    assert!(extra > 0.0);
    assert_eq!(nested_mask, 0x07);
    assert_eq!(session.represented_rest_time_secs_like_cpp, 1_010);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), extra);
    assert_eq!(
        session.represented_xp_rest_state_like_cpp(),
        REST_STATE_RESTED_LIKE_CPP
    );
}
#[test]
fn online_rest_tick_only_accrues_when_cpp_three_percent_gate_passes() {
    let (mut session, _, _) = make_session();
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    session.set_player_next_level_xp_like_cpp(72_000);
    session.load_represented_xp_rest_bonus_like_cpp(REST_STATE_NORMAL_LIKE_CPP, 0.0);
    assert!(session.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0));
    session.represented_rest_time_secs_like_cpp = 1_000;

    session.tick_represented_online_xp_rest_bonus_with_roll_like_cpp(1_010, false);
    assert_eq!(session.represented_rest_time_secs_like_cpp, 1_000);
    assert_eq!(session.represented_xp_rest_bonus_like_cpp(), 0.0);

    session.tick_represented_online_xp_rest_bonus_with_roll_like_cpp(1_010, true);
    assert_eq!(session.represented_rest_time_secs_like_cpp, 1_010);
    assert!(session.represented_xp_rest_bonus_like_cpp() > 0.0);
}
#[tokio::test]
async fn area_trigger_tavern_validates_db2_trigger_and_position_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(42);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();

    session.handle_area_trigger(pkt).await;

    assert!(session.represented_is_resting_like_cpp());
}
#[tokio::test]
async fn area_trigger_tavern_rejects_spoofed_far_enter_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(100.0, 20.0, 30.0, 0.0));
    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(42);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();

    session.handle_area_trigger(pkt).await;

    assert!(!session.represented_is_resting_like_cpp());
}
#[tokio::test]
async fn area_trigger_tavern_accepts_inside_leave_without_radius_check_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));

    let mut enter = WorldPacket::new_empty();
    enter.write_uint32(42);
    enter.write_bit(true);
    enter.write_bit(false);
    enter.flush_bits();
    session.handle_area_trigger(enter).await;
    assert!(session.represented_is_resting_like_cpp());

    let mut inside_leave = WorldPacket::new_empty();
    inside_leave.write_uint32(42);
    inside_leave.write_bit(false);
    inside_leave.write_bit(false);
    inside_leave.flush_bits();
    session.handle_area_trigger(inside_leave).await;

    assert!(!session.represented_is_resting_like_cpp());
}
#[tokio::test]
async fn area_trigger_tavern_is_ignored_during_taxi_flight_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));
    session.set_taxi_flight_state_like_cpp(
        RepresentedTaxiFlightNodeLikeCpp {
            map_id: 1,
            position: Position::new(11.0, 20.0, 30.0, 0.0),
            teleport_flag: false,
        },
        None,
    );

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(42);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    session.handle_area_trigger(pkt).await;

    assert!(!session.represented_is_resting_like_cpp());
}
#[tokio::test]
async fn truncated_area_trigger_packet_cannot_enter_tavern_like_cpp() {
    let (mut session, _, _) = make_session();
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));

    let mut truncated = WorldPacket::new_empty();
    truncated.write_uint32(42);
    session.handle_area_trigger(truncated).await;

    assert!(!session.represented_is_resting_like_cpp());
}
#[tokio::test]
async fn area_trigger_tavern_respects_client_triggered_conditions_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xA7C0);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(1, 1, 8, 10, 0);
    let outcome = wow_data::TavernAreaTriggerStoreLikeCpp::from_ids_like_cpp([42], |_| true);
    session.set_tavern_area_trigger_store(Arc::new(outcome.store));
    session.set_area_trigger_db2_store(Arc::new(wow_data::AreaTriggerDb2Store::from_entries([
        test_db2_area_trigger_like_cpp(42, 1, Position::new(10.0, 20.0, 30.0, 0.0)),
    ])));
    session.set_player_map_position_like_cpp(1, Position::new(11.0, 20.0, 30.0, 0.0));
    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::AreaTriggerClientTriggered,
            source_group: 0,
            source_entry: 42,
            source_id: 0,
            condition_type: ConditionType::MapId,
            condition_value1: 999_999,
            ..Condition::default()
        }]),
    ));

    let mut pkt = WorldPacket::new_empty();
    pkt.write_uint32(42);
    pkt.write_bit(true);
    pkt.write_bit(false);
    pkt.flush_bits();
    session.handle_area_trigger(pkt).await;

    assert!(!session.represented_is_resting_like_cpp());

    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::AreaTriggerClientTriggered,
            source_group: 0,
            source_entry: 42,
            source_id: 0,
            condition_type: ConditionType::WorldState,
            condition_value1: 123,
            condition_value2: 1,
            negative_condition: true,
            ..Condition::default()
        }]),
    ));
    let mut unsupported = WorldPacket::new_empty();
    unsupported.write_uint32(42);
    unsupported.write_bit(true);
    unsupported.write_bit(false);
    unsupported.flush_bits();
    session.handle_area_trigger(unsupported).await;

    assert!(
        !session.represented_is_resting_like_cpp(),
        "a valid negative condition without represented map state must fail closed before negation"
    );

    session.set_condition_store(Arc::new(
        ConditionEntriesByTypeStore::from_conditions_like_cpp([Condition {
            source_type: ConditionSourceType::AreaTriggerClientTriggered,
            source_group: 0,
            source_entry: 42,
            source_id: 0,
            condition_type: ConditionType::None,
            script_id: 7,
            ..Condition::default()
        }]),
    ));
    let mut scripted_condition = WorldPacket::new_empty();
    scripted_condition.write_uint32(42);
    scripted_condition.write_bit(true);
    scripted_condition.write_bit(false);
    scripted_condition.flush_bits();
    session.handle_area_trigger(scripted_condition).await;

    assert!(
        !session.represented_is_resting_like_cpp(),
        "a ConditionScript row must not bypass its unrepresented OnConditionCheck callback"
    );
}
