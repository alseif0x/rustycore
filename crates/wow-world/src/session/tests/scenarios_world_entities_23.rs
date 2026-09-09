//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_aggro_tick_once_respects_no_gray_aggro_config_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_018);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let gray_player = ObjectGuid::create_player(1, 91_019);
    let allowed_player = ObjectGuid::create_player(1, 91_020);
    let mut allowed_candidate =
        legacy_aggro_candidate_like_cpp(allowed_player, Position::new(10.5, 10.5, 0.0, 0.0));
    allowed_candidate.player_gray_level = 24;
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(gray_player, Position::new(10.5, 10.5, 0.0, 0.0)),
        allowed_candidate,
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        LegacyCreatureAggroConfigLikeCpp {
            no_gray_aggro_above: 80,
            no_gray_aggro_below: 0,
            ..legacy_aggro_hostile_config_like_cpp()
        },
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 2);
    assert_eq!(outcome.gray_aggro_rejections, 1);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, allowed_player);
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, Some(allowed_player));
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_untargetable_players_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_021);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let good_player = ObjectGuid::create_player(1, 91_026);
    let mut non_attackable = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_022),
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    non_attackable.player_unit_flags |= UnitFlags::NON_ATTACKABLE.bits();
    let mut immune_to_npc = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_023),
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    immune_to_npc.player_unit_flags |= UnitFlags::IMMUNE_TO_NPC.bits();
    let mut fake_dead = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_024),
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    fake_dead.player_unit_state |= UnitState::DIED.bits();
    let mut game_master = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_025),
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    game_master.player_is_game_master = true;
    let mut in_flight = legacy_aggro_candidate_like_cpp(
        ObjectGuid::create_player(1, 91_027),
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    in_flight.player_unit_state |= UnitState::IN_FLIGHT.bits();
    let candidates = vec![
        non_attackable,
        immune_to_npc,
        fake_dead,
        game_master,
        in_flight,
        legacy_aggro_candidate_like_cpp(good_player, Position::new(10.5, 10.5, 0.0, 0.0)),
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 6);
    assert_eq!(outcome.targetability_rejections, 5);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, good_player);
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, Some(good_player));
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_invisible_player_without_detection_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_034);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let invisible_player = ObjectGuid::create_player(1, 91_035);
    let visible_player = ObjectGuid::create_player(1, 91_036);
    let mut invisible_candidate =
        legacy_aggro_candidate_like_cpp(invisible_player, Position::new(10.5, 10.5, 0.0, 0.0));
    let mut invisible_unit = Unit::new(true);
    invisible_unit.set_invisibility_like_cpp(0, 100);
    invisible_candidate.player_visibility_detection =
        invisible_unit.visibility_detection_like_cpp().clone();

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[
            invisible_candidate,
            legacy_aggro_candidate_like_cpp(visible_player, Position::new(10.5, 10.5, 0.0, 0.0)),
        ],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.visibility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, visible_player);
}
#[test]
fn legacy_creature_aggro_tick_once_alerts_stealthed_player_without_aggro_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_041);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 9.0;
            creature.creature.unit_mut().set_level(80);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let stealthed_player = ObjectGuid::create_player(1, 91_042);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(stealthed_player, Position::new(17.0, 10.0, 0.0, 0.0));
    let mut stealthed_unit = Unit::new(true);
    stealthed_unit.set_stealth_like_cpp(0, 408);
    candidate.player_visibility_detection = stealthed_unit.visibility_detection_like_cpp().clone();

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[candidate],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.visibility_rejections, 1);
    assert_eq!(outcome.alert_triggers, 1);
    assert_eq!(outcome.alert_rejections, 0);
    assert_eq!(outcome.plan.events.len(), 1);
    let alert_event = &outcome.plan.events[0];
    assert_eq!(alert_event.source_guid, creature_guid);
    assert!(matches!(
        &alert_event.recipients,
        crate::map_manager::RecipientRule::MapBroadcastVisible {
            map_id: 0,
            instance_id: 0
        }
    ));
    let mut alert_packet = wow_packet::WorldPacket::from_bytes(&alert_event.packet_bytes);
    assert_eq!(
        alert_packet.read_uint16().expect("opcode"),
        ServerOpcodes::AiReaction as u16
    );
    assert_eq!(
        alert_packet.read_packed_guid().expect("unit guid"),
        creature_guid
    );
    assert_eq!(
        alert_packet.read_uint32().expect("reaction"),
        wow_constants::creature::AiReaction::Alert as u32
    );
    assert!(alert_packet.is_empty());
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let (combat_target, has_distract_generator) = {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        (
            creature.creature.ai_ownership().combat_target,
            creature
                .creature
                .unit()
                .subsystems()
                .motion
                .active_generators
                .iter()
                .any(|generator| generator.kind == wow_entities::MovementGeneratorKind::Distract),
        )
    };
    assert_eq!(combat_target, None);
    assert!(has_distract_generator);
}
#[test]
fn legacy_creature_aggro_tick_once_fails_closed_without_visibility_snapshot_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_039);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_040);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0));
    candidate.player_visibility_represented = false;

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[candidate],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.visibility_unrepresented, 1);
    assert_eq!(outcome.visibility_rejections, 0);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_allows_invisible_player_when_creature_detects_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_037);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature
                .creature
                .unit_mut()
                .set_invisibility_detect_like_cpp(0, 100);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let invisible_player = ObjectGuid::create_player(1, 91_038);
    let mut invisible_candidate =
        legacy_aggro_candidate_like_cpp(invisible_player, Position::new(10.5, 10.5, 0.0, 0.0));
    let mut invisible_unit = Unit::new(true);
    invisible_unit.set_invisibility_like_cpp(0, 100);
    invisible_candidate.player_visibility_detection =
        invisible_unit.visibility_detection_like_cpp().clone();

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[invisible_candidate],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.visibility_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, invisible_player);
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_reputation_without_at_war_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_027);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let rejected_player = ObjectGuid::create_player(1, 91_028);
    let allowed_player = ObjectGuid::create_player(1, 91_029);
    let mut rejected =
        legacy_aggro_candidate_like_cpp(rejected_player, Position::new(10.5, 10.5, 0.0, 0.0));
    rejected.player_faction_template_id = 1;
    rejected.player_reputation_standings = vec![(72, -6_000)];
    rejected.player_reputation_state_flags = vec![(72, 0)];
    let mut allowed =
        legacy_aggro_candidate_like_cpp(allowed_player, Position::new(10.5, 10.5, 0.0, 0.0));
    allowed.player_faction_template_id = 1;
    allowed.player_reputation_standings = vec![(72, -6_000)];
    allowed.player_reputation_state_flags =
        vec![(72, wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP)];
    let config = legacy_aggro_relation_config_like_cpp(
        faction_template_entry(14, 72, 0, 0, 0),
        faction_template_entry(1, 930, 0, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    );
    let candidates = vec![rejected, allowed];

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.candidates_seen, 2);
    assert_eq!(outcome.hostility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, allowed_player);
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, Some(allowed_player));
}
#[test]
fn legacy_creature_aggro_tick_once_honors_forced_reputation_rank_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_034);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let friendly_forced_player = ObjectGuid::create_player(1, 91_035);
    let hostile_forced_player = ObjectGuid::create_player(1, 91_036);
    let mut friendly_forced = legacy_aggro_candidate_like_cpp(
        friendly_forced_player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    );
    friendly_forced.player_forced_reputation_ranks =
        vec![(72, wow_data::reputation::ReputationRankLikeCpp::Friendly)];
    let mut hostile_forced =
        legacy_aggro_candidate_like_cpp(hostile_forced_player, Position::new(10.5, 10.5, 0.0, 0.0));
    hostile_forced.player_forced_reputation_ranks =
        vec![(72, wow_data::reputation::ReputationRankLikeCpp::Hostile)];
    let candidates = vec![friendly_forced, hostile_forced];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.hostility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, hostile_forced_player);
}
#[test]
fn legacy_creature_aggro_tick_once_ignores_reputation_when_unit_flag2_says_so_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_037);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_038);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0));
    candidate.player_unit_flags2 = UnitFlags2::IGNORE_REPUTATION.bits();
    candidate.player_reputation_standings = vec![(72, -6_000)];
    candidate.player_reputation_state_flags = vec![(72, 0)];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[candidate],
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.hostility_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_unrepresented_faction_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_039);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_040);
    let candidate = legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0));

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[candidate],
        LegacyCreatureAggroConfigLikeCpp::default(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.hostility_unrepresented, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_friendly_faction_templates_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_030);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_031);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0));
    candidate.player_faction_template_id = 1;
    let config = legacy_aggro_relation_config_like_cpp(
        faction_template_entry(14, 72, 0, 1, 0),
        faction_template_entry(1, 930, 1, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    );

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &[candidate], config);

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.hostility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_prefers_hostile_static_reaction_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_032);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_033);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(player, Position::new(10.5, 10.5, 0.0, 0.0));
    candidate.player_faction_template_id = 1;
    let config = legacy_aggro_relation_config_like_cpp(
        wow_data::progression_rewards::FactionTemplateEntry {
            id: 14,
            faction: 72,
            flags: 0,
            faction_group: 0,
            friend_group: 1,
            enemy_group: 1,
            enemies: [0; 8],
            friend: [0; 8],
        },
        faction_template_entry(1, 930, 1, 0, 0),
        FactionEntry::for_test_like_cpp(72, 1),
    );

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &[candidate], config);

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.hostility_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_tick_once_respects_react_state_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_010);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .creature
                .set_react_state(wow_entities::ReactState::Passive);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_011);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
    let combat_target = {
        let guard = manager.read().unwrap();
        guard
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target
    };
    assert_eq!(combat_target, None);
}
#[test]
fn legacy_creature_aggro_tick_once_suppresses_empty_los_ai_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_112);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("ReactorAI", String::new());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_113);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.ai_los_suppressed, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_fails_closed_for_script_ai_registry_gap_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_114);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp(String::new(), "npc_scripted_ai");
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_115);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.ai_selection_unrepresented, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn creature_ai_spell_target_restrictions_require_hostile_unit_only_wire_like_cpp() {
    const SPELL_ID: u32 = 70_189;
    let rejects = |targets: i32, target_creature_type: i16| {
        let config = LegacyCreatureAggroConfigLikeCpp {
            spell_target_restrictions_store: Some(Arc::new(
                wow_data::SpellTargetRestrictionsStore::from_entries([
                    wow_data::SpellTargetRestrictionsEntry {
                        id: 1,
                        difficulty_id: 0,
                        cone_degrees: 0.0,
                        max_targets: 0,
                        max_target_level: 0,
                        target_creature_type,
                        targets,
                        width: 0.0,
                        spell_id: SPELL_ID,
                    },
                ]),
            )),
            ..Default::default()
        };
        creature_ai_spell_has_unrepresented_target_restrictions_like_cpp(SPELL_ID, 0, &config)
    };

    for targets in [0, 0x0000_0002, 0x0000_0080, 0x0000_0082] {
        assert!(!rejects(targets, 0), "represented target mask {targets:#x}");
    }
    for targets in [
        0x0000_0004,
        0x0000_0008,
        0x0000_0010,
        0x0000_0020,
        0x0000_0040,
        0x0000_0100,
        0x0000_0400,
        0x0000_0800,
        0x0001_0000,
        0x0010_0000,
        i32::MIN,
    ] {
        assert!(
            rejects(targets, 0),
            "unrepresented target mask {targets:#x}"
        );
    }
    assert!(rejects(0, 1), "creature-type masks remain fail-closed");
}
#[test]
fn creature_ai_spell_peaceful_only_attribute_is_rejected_in_combat_like_cpp() {
    const SPELL_ID: u32 = 70_188;
    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(SPELL_ID as i32, 6, 0),
        false,
        30.0,
    );
    assert!(!creature_ai_spell_is_combat_forbidden_like_cpp(
        SPELL_ID, 0, &config
    ));
    let mut attributes = represented_creature_spell_test_attributes_like_cpp(true);
    attributes[0] |= wow_data::spell::attributes::SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL;
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_misc_attributes_like_cpp(SPELL_ID as i32, attributes);
    assert!(creature_ai_spell_is_combat_forbidden_like_cpp(
        SPELL_ID, 0, &config
    ));
}
#[test]
fn creature_ai_spell_shapeshift_masks_fail_closed_like_cpp() {
    const SPELL_ID: u32 = 70_187;
    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(SPELL_ID as i32, 6, 0),
        false,
        30.0,
    );
    assert!(config.spell_has_no_unrepresented_shapeshift_requirements_like_cpp(SPELL_ID));
    Arc::get_mut(config.spell_store.as_mut().unwrap())
        .unwrap()
        .insert_spell_shapeshift_masks_like_cpp(SPELL_ID as i32, 1 << 4, 0);
    assert!(
        !config.spell_has_no_unrepresented_shapeshift_requirements_like_cpp(SPELL_ID),
        "non-neutral C++ stance authority must not reach START/GO"
    );
    config.spell_store = None;
    assert!(
        !config.spell_has_no_unrepresented_shapeshift_requirements_like_cpp(SPELL_ID),
        "missing shapeshift authority remains fail-closed"
    );
}
#[test]
fn creature_ai_spell_ignore_los_attribute_uses_exact_cpp_bit() {
    let mut attributes = represented_creature_spell_test_attributes_like_cpp(true);
    assert_eq!(
        attributes[2] & wow_data::spell::attributes::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT,
        0
    );
    attributes[2] |= wow_data::spell::attributes::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT;
    assert_ne!(
        attributes[2] & wow_data::spell::attributes::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT,
        0
    );
    assert_eq!(
        wow_data::spell::attributes::SPELL_ATTR2_IGNORE_LINE_OF_SIGHT,
        0x0000_0004
    );
}
