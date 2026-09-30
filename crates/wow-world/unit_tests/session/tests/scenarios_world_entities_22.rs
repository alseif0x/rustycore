//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_aggro_tick_once_skips_sightless_creature_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_096);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 50.0;
            creature.creature.unit_mut().set_level(25);
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::STUNNED.bits());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_097);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.sightless_creatures_skipped, 1);
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
fn legacy_creature_aggro_rejects_water_target_when_creature_cannot_enter_water_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_064);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 50.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_swim_allowed_runtime_like_cpp(false);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let water_player = ObjectGuid::create_player(1, 91_065);
    let dry_player = ObjectGuid::create_player(1, 91_066);
    let mut water_candidate =
        legacy_aggro_candidate_like_cpp(water_player, Position::new(10.5, 10.5, 0.0, 0.0));
    water_candidate.player_liquid_status_like_cpp = LIQUID_MAP_IN_WATER_LIKE_CPP;
    let candidates = vec![
        water_candidate,
        legacy_aggro_candidate_like_cpp(dry_player, Position::new(10.5, 10.5, 0.0, 0.0)),
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.accessibility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, dry_player);
}
#[test]
fn legacy_creature_aggro_rejects_land_target_when_creature_cannot_walk_or_fly_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_067);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 50.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_ground_movement_type_runtime_like_cpp(
                wow_constants::CreatureGroundMovementType::None as u8,
            );
            creature.creature.set_swim_allowed_runtime_like_cpp(true);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_068);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.accessibility_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_home_range_uses_map_visibility_category_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_029);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_030);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(130.0, 10.0, 0.0, 0.0),
    )];
    let mut config = legacy_aggro_hostile_config_with_rate_like_cpp(6.0);
    config.map_store = Some(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_BATTLEGROUND,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);

    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_home_range_is_skipped_in_dungeon_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_031);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_032);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];
    let mut config = legacy_aggro_hostile_config_with_rate_like_cpp(8.0);
    config.map_store = Some(Arc::new(wow_data::MapStore::from_entries([
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

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);

    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_player_owned_creature_requires_owner_position_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_033);
    let owner = ObjectGuid::create_player(1, 91_034);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
            creature
                .creature
                .set_last_damaged_time_like_cpp(wow_entities::game_time_secs_like_cpp() + 10);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_type_like_cpp(
                    wow_entities::AppliedAuraRef::new(35_517, owner, 0, 0x1),
                    wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_035);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];
    let mut config = legacy_aggro_hostile_config_with_rate_like_cpp(8.0);
    config.map_store = Some(Arc::new(wow_data::MapStore::from_entries([
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

    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);

    assert_eq!(outcome.owner_position_unrepresented, 1);
    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_player_owned_creature_uses_owner_position_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_048);
    let owner = ObjectGuid::create_player(1, 91_049);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_050);
    let mut victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(301.5, 10.0, 0.0, 0.0));
    victim_candidate.player_combat_reach = 1.0;
    let mut owner_candidate =
        legacy_aggro_candidate_like_cpp(owner, Position::new(200.0, 10.0, 0.0, 0.0));
    owner_candidate.player_combat_reach = 1.0;
    owner_candidate.player_unit_flags |= UnitFlags::NON_ATTACKABLE.bits();
    let candidates = vec![victim_candidate, owner_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(9.0),
    );

    assert_eq!(outcome.owner_position_unrepresented, 0);
    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, victim);
}
#[test]
fn legacy_creature_aggro_player_owned_creature_rejects_beyond_owner_position_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_051);
    let owner = ObjectGuid::create_player(1, 91_052);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_053);
    let mut victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(303.5, 10.0, 0.0, 0.0));
    victim_candidate.player_combat_reach = 1.0;
    let mut owner_candidate =
        legacy_aggro_candidate_like_cpp(owner, Position::new(200.0, 10.0, 0.0, 0.0));
    owner_candidate.player_combat_reach = 1.0;
    owner_candidate.player_unit_flags |= UnitFlags::NON_ATTACKABLE.bits();
    let candidates = vec![victim_candidate, owner_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(7.0),
    );

    assert_eq!(outcome.owner_position_unrepresented, 0);
    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_player_owned_creature_rejects_exact_owner_distance_edge_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_057);
    let owner = ObjectGuid::create_player(1, 91_058);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_059);
    let mut victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(302.0, 10.0, 0.0, 0.0));
    victim_candidate.player_combat_reach = 1.0;
    let mut owner_candidate =
        legacy_aggro_candidate_like_cpp(owner, Position::new(200.0, 10.0, 0.0, 0.0));
    owner_candidate.player_combat_reach = 1.0;
    owner_candidate.player_unit_flags |= UnitFlags::NON_ATTACKABLE.bits();
    let candidates = vec![victim_candidate, owner_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(6.0),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_player_owned_creature_wrong_map_owner_is_unrepresented_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_054);
    let owner = ObjectGuid::create_player(1, 91_055);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_056);
    let victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(80.0, 10.0, 0.0, 0.0));
    let mut owner_candidate =
        legacy_aggro_candidate_like_cpp(owner, Position::new(80.0, 10.0, 0.0, 0.0));
    owner_candidate.map_id = 1;
    owner_candidate.player_unit_flags |= UnitFlags::NON_ATTACKABLE.bits();
    let candidates = vec![victim_candidate, owner_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(6.0),
    );

    assert_eq!(outcome.owner_position_unrepresented, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_creature_owned_creature_uses_owner_position_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_122);
    let owner = test_creature_guid(91_123);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    register_test_creature(&mut session, manager.clone(), owner, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    session
        .mutate_world_creature(owner, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(1.0);
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::SIGHTLESS.bits());
            creature
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::new(200.0, 10.0, 0.0, 0.0));
            creature
                .creature
                .set_ai_position(Position::new(200.0, 10.0, 0.0, 0.0));
            creature
                .creature
                .set_ai_home_position(Position::new(200.0, 10.0, 0.0, 0.0));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_124);
    let mut victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(301.5, 10.0, 0.0, 0.0));
    victim_candidate.player_combat_reach = 1.0;
    let candidates = vec![victim_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.owner_position_unrepresented, 0);
    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, victim);
}
#[test]
fn legacy_creature_aggro_creature_owned_creature_rejects_beyond_owner_position_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_125);
    let owner = test_creature_guid(91_126);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    register_test_creature(&mut session, manager.clone(), owner, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 400.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .control
                .set_owner_guid(Some(owner));
        })
        .unwrap();
    session
        .mutate_world_creature(owner, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(1.0);
            creature
                .creature
                .unit_mut()
                .add_unit_state(UnitState::SIGHTLESS.bits());
            creature
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::new(200.0, 10.0, 0.0, 0.0));
            creature
                .creature
                .set_ai_position(Position::new(200.0, 10.0, 0.0, 0.0));
            creature
                .creature
                .set_ai_home_position(Position::new(200.0, 10.0, 0.0, 0.0));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim = ObjectGuid::create_player(1, 91_127);
    let mut victim_candidate =
        legacy_aggro_candidate_like_cpp(victim, Position::new(303.5, 10.0, 0.0, 0.0));
    victim_candidate.player_combat_reach = 1.0;
    let candidates = vec![victim_candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.owner_position_unrepresented, 0);
    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_recent_damage_skips_home_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_036);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            let _ = creature.creature.take_ai_damage(1, 0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_037);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_expired_recent_damage_uses_home_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_044);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .set_last_damaged_time_like_cpp(wow_entities::game_time_secs_like_cpp() - 1);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_045);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_taunt_skips_home_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_038);
    let caster = ObjectGuid::create_player(1, 91_039);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_type_like_cpp(
                    wow_entities::AppliedAuraRef::new(35_517, caster, 0, 0x1),
                    wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_040);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_world_boss_does_not_skip_home_range_for_taunt_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_041);
    let caster = ObjectGuid::create_player(1, 91_042);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .set_type_flags_runtime_like_cpp(CreatureTypeFlags::BOSS_MOB.bits());
            creature
                .creature
                .set_last_damaged_time_like_cpp(wow_entities::game_time_secs_like_cpp() + 10);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_type_like_cpp(
                    wow_entities::AppliedAuraRef::new(35_517, caster, 0, 0x1),
                    wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_043);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(240.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(8.0),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_home_range_uses_template_flight_2d_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let dynamic_guid = test_creature_guid(91_025);
    let template_guid = test_creature_guid(91_026);
    register_test_creature(&mut session, manager.clone(), dynamic_guid, 25);
    register_test_creature(&mut session, manager.clone(), template_guid, 25);
    session
        .mutate_world_creature(dynamic_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature
                .creature
                .set_movement_flags_runtime_like_cpp(MovementFlag::DISABLE_GRAVITY);
        })
        .unwrap();
    session
        .mutate_world_creature(template_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_flight_movement_type_runtime_like_cpp(
                wow_constants::CreatureFlightMovementType::CanFly as u8,
            );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let dynamic_player = ObjectGuid::create_player(1, 91_027);
    let template_player = ObjectGuid::create_player(1, 91_028);
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(dynamic_player, Position::new(10.5, 10.5, 4.0, 0.0)),
        legacy_aggro_candidate_like_cpp(template_player, Position::new(10.5, 10.5, 4.0, 0.0)),
    ];

    let mut config = legacy_aggro_hostile_config_with_rate_like_cpp(8.0);
    config.visibility_distance_continents = 3.0;
    let outcome =
        run_legacy_creature_aggro_tick_once_with_config_like_cpp(&manager, &candidates, config);

    assert_eq!(outcome.home_range_rejections, 2);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].attacker_guid, template_guid);
    assert_eq!(outcome.commands[0].victim_guid, dynamic_player);
}
