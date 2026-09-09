//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn legacy_creature_call_assistance_rejects_blocked_static_vmap_los_like_cpp() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::map_manager::{LiveTerrainHeights, RuntimeTickOwner};

    #[derive(Debug, Default)]
    struct BlockingAssistanceLos {
        calls: AtomicUsize,
    }

    impl wow_map::StaticVMapLineOfSightProvider for BlockingAssistanceLos {
        fn is_in_line_of_sight(&self, _query: wow_map::VMapLineOfSightQuery) -> bool {
            self.calls.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    let manager = shared_map_manager();
    let provider = Arc::new(BlockingAssistanceLos::default());
    let shared_provider: wow_map::SharedStaticVMapLineOfSightProvider = provider.clone();
    manager.write().unwrap().set_terrain(Arc::new(
        LiveTerrainHeights::new_with_static_vmap_line_of_sight(
            "/tmp/rustycore-assistance-los-test",
            shared_provider,
        ),
    ));
    let (mut session, _, _) = make_session();
    let caller_guid = test_creature_guid(91_222);
    let assistant_guid = test_creature_guid(91_223);
    let victim = ObjectGuid::create_player(1, 91_224);
    register_test_creature(&mut session, manager.clone(), caller_guid, 100);
    register_test_creature(&mut session, manager.clone(), assistant_guid, 100);
    session
        .mutate_world_creature(caller_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
        })
        .unwrap();
    session
        .mutate_world_creature(assistant_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 0.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_faction(14);
            creature
                .creature
                .unit_mut()
                .world_mut()
                .relocate(Position::new(12.0, 10.0, 0.0, 0.0));
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[legacy_aggro_candidate_like_cpp(
            victim,
            Position::new(10.5, 10.5, 0.0, 0.0),
        )],
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.assistance_scheduled, 0);
    assert_eq!(provider.calls.load(Ordering::Relaxed), 1);
}
#[test]
fn legacy_creature_overlapping_assistance_uses_later_valid_event_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let assistant_guid = test_creature_guid(91_225);
    let first_caller_guid = test_creature_guid(91_226);
    let second_caller_guid = test_creature_guid(91_227);
    let missing_first_victim = ObjectGuid::create_player(1, 91_228);
    let valid_second_victim = ObjectGuid::create_player(1, 91_229);
    for guid in [assistant_guid, first_caller_guid, second_caller_guid] {
        register_test_creature(&mut session, manager.clone(), guid, 100);
        session
            .mutate_world_creature(guid, |creature| {
                creature.creature.ai_ownership_mut().aggro_radius = 0.0;
                creature.creature.unit_mut().set_level(25);
                creature.creature.set_faction(14);
            })
            .unwrap();
    }
    session
        .mutate_world_creature(first_caller_guid, |caller| {
            assert!(caller.schedule_assistance_like_cpp(
                missing_first_victim,
                vec![assistant_guid],
                1_500,
            ));
            caller.backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
        })
        .unwrap();
    session
        .mutate_world_creature(second_caller_guid, |caller| {
            assert!(caller.schedule_assistance_like_cpp(
                valid_second_victim,
                vec![assistant_guid],
                1_500,
            ));
            caller.backdate_runtime_clock_for_test(ASSISTANCE_DELAY_ELAPSED_LIKE_CPP);
        })
        .unwrap();
    session
        .mutate_world_creature(assistant_guid, |assistant| {
            assistant.enter_combat(missing_first_victim);
            assert_eq!(
                assistant.take_assistance_call_like_cpp(),
                Some(missing_first_victim),
                "C++ does not SetNoCallAssistance until a delayed event revalidates and executes"
            );
            let _ = assistant.reset_combat();
        })
        .unwrap();
    assert!(
        manager
            .write()
            .unwrap()
            .remove_creature_any(0, 0, assistant_guid)
            .is_some()
    );
    register_test_creature(&mut session, manager.clone(), assistant_guid, 100);
    session
        .mutate_world_creature(assistant_guid, |assistant| {
            assistant.creature.ai_ownership_mut().aggro_radius = 0.0;
            assistant.creature.unit_mut().set_level(25);
            assistant.creature.set_faction(14);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[legacy_aggro_candidate_like_cpp(
            valid_second_victim,
            Position::new(10.5, 10.5, 0.0, 0.0),
        )],
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.assistance_starts, 1);
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, assistant_guid)
            .unwrap()
            .creature
            .ai_ownership()
            .combat_target,
        Some(valid_second_victim),
        "C++ caller-owned AssistDelayEvents resolve an assistant GUID after its runtime object is replaced"
    );
}
#[test]
fn legacy_creature_aggro_preserves_high_priority_point_spline_above_chase_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_210);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature
                .begin_move_spline_like_cpp(Position::new(20.0, 10.0, 0.0, 0.0))
                .expect("launch high-priority point spline");
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .motion
                .move_charge(42);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_211);
    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &[legacy_aggro_candidate_like_cpp(
            player,
            Position::new(10.5, 10.5, 0.0, 0.0),
        )],
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.movement_interrupts, 0);
    assert_eq!(outcome.plan.events.len(), 1);
    assert!(matches!(
        outcome.plan.events[0].recipients,
        crate::map_manager::RecipientRule::NearbyVisibleDurable { .. }
    ));
    let (runtime_kind, represented_kind, has_active_spline) = {
        let guard = manager.read().unwrap();
        let creature = guard.find_creature(0, 0, creature_guid).unwrap();
        (
            creature.runtime_motion_master_current_kind_like_cpp(),
            creature
                .creature
                .unit()
                .subsystems()
                .motion
                .current_movement_generator()
                .kind,
            creature.active_move_spline_like_cpp().is_some(),
        )
    };
    assert_eq!(
        runtime_kind,
        Some(wow_movement::MovementGeneratorType::Point)
    );
    assert_eq!(represented_kind, MovementGeneratorKind::Point);
    assert!(has_active_spline);
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_excessive_z_distance_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_018);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 5.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let high_player = ObjectGuid::create_player(1, 91_019);
    let allowed_player = ObjectGuid::create_player(1, 91_020);
    let mut allowed_by_reach =
        legacy_aggro_candidate_like_cpp(allowed_player, Position::new(10.5, 10.5, 4.0, 0.0));
    allowed_by_reach.player_combat_reach = 1.0;
    let candidates = vec![
        legacy_aggro_candidate_like_cpp(high_player, Position::new(10.5, 10.5, 3.1, 0.0)),
        allowed_by_reach,
    ];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert!(!outcome.skipped_owner_not_global);
    assert_eq!(outcome.maps_seen, 1);
    assert_eq!(outcome.creatures_seen, 1);
    assert_eq!(outcome.candidates_seen, 2);
    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].attacker_guid, creature_guid);
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
fn legacy_creature_aggro_tick_once_honors_can_fly_z_exemption_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_021);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 50.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_flight_movement_type_runtime_like_cpp(
                wow_constants::CreatureFlightMovementType::DisableGravity as u8,
            );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_022);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 3.1, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_with_rate_like_cpp(3.0),
    );

    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].attacker_guid, creature_guid);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_tick_once_uses_get_attack_distance_not_fixed_radius_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_128);
    register_test_creature(&mut session, manager.clone(), creature_guid, 80);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(0.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_129);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(56.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.home_range_rejections, 0);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_uses_detect_range_aura_modifiers_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_134);
    register_test_creature(&mut session, manager.clone(), creature_guid, 75);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(75);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(0.0);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_modifier_like_cpp(
                    wow_entities::AppliedAuraRef::new(91_134, creature_guid, 0, 0x1),
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE,
                    3,
                );
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_135);
    let mut candidate =
        legacy_aggro_candidate_like_cpp(player, Position::new(29.0, 10.0, 0.0, 0.0));
    candidate.player_detected_range_aura_mod = 2.0;
    let candidates = vec![candidate];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(
        outcome.aggro_starts, 1,
        "C++ Creature::GetAttackDistance adds creature detect range and player detected range auras before clamping"
    );
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[tokio::test]
async fn check_creature_aggro_uses_detected_range_aura_modifier_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 91_136);
    let creature_guid = test_creature_guid(91_137);
    session.set_player_guid(Some(player));
    session.set_player_level_like_cpp(80);
    session.set_player_position_like_cpp(Position::new(29.0, 10.0, 0.0, 0.0));
    register_test_creature(&mut session, manager, creature_guid, 75);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(75);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(0.0);
        })
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            91_136,
            player,
            &wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE,
                effect_base_points: 4,
                ..Default::default()
            },
            RepresentedAuraEffectLikeCpp::ModDetectedRange,
            30_000,
        )
        .unwrap();

    session.check_creature_aggro().await;

    assert!(session.in_combat);
    assert_eq!(session.combat_target, Some(creature_guid));
    assert!(drain_server_opcodes(&send_rx).contains(&ServerOpcodes::AttackStart));
}
#[tokio::test]
async fn check_creature_aggro_uses_configured_aggro_rate_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 91_138);
    let creature_guid = test_creature_guid(91_139);
    session.set_player_guid(Some(player));
    session.set_player_level_like_cpp(80);
    session.set_player_position_like_cpp(Position::new(10.5, 10.0, 0.0, 0.0));
    session.set_legacy_creature_aggro_config_like_cpp(LegacyCreatureAggroConfigLikeCpp {
        creature_aggro_rate: 0.0,
        max_player_level_config: 80,
        ..Default::default()
    });
    register_test_creature(&mut session, manager, creature_guid, 80);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(0.0);
        })
        .unwrap();

    session.check_creature_aggro().await;

    assert!(!session.in_combat);
    assert_ne!(session.combat_target, Some(creature_guid));
    assert!(!drain_server_opcodes(&send_rx).contains(&ServerOpcodes::AttackStart));
}
#[tokio::test]
async fn check_creature_aggro_uses_template_required_expansion_cap_like_cpp() {
    let manager = shared_map_manager();
    let (mut session, _, send_rx) = make_session();
    let player = ObjectGuid::create_player(1, 91_140);
    let creature_guid = test_creature_guid(91_141);
    session.set_player_guid(Some(player));
    session.set_player_level_like_cpp(80);
    session.set_player_position_like_cpp(Position::new(27.0, 10.0, 0.0, 0.0));
    session.set_legacy_creature_aggro_config_like_cpp(LegacyCreatureAggroConfigLikeCpp {
        creature_aggro_rate: 1.0,
        max_player_level_config: 80,
        ..Default::default()
    });
    register_test_creature(&mut session, manager, creature_guid, 80);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(0.0);
            creature.creature.set_required_expansion_runtime_like_cpp(1);
        })
        .unwrap();

    session.check_creature_aggro().await;

    assert!(
        !session.in_combat,
        "C++ Creature::GetAttackDistance caps level 80 RequiredExpansion=1 creatures to expansion max level 70"
    );
    assert_ne!(session.combat_target, Some(creature_guid));
    assert!(!drain_server_opcodes(&send_rx).contains(&ServerOpcodes::AttackStart));
}
#[test]
fn legacy_creature_aggro_tick_once_uses_template_required_expansion_cap_like_cpp() {
    use crate::map_manager::{RuntimeTickOwner, WorldCreature};
    use wow_entities::{
        CreatureCreateLifecycleRecord, CreatureLifecycleStats, CreatureSpawnLifecycleRecord,
        CreatureTemplateLifecycleRecord, MovementGeneratorType,
    };

    let manager = shared_map_manager();
    let creature_guid = test_creature_guid(91_132);
    let spells = [0; wow_entities::MAX_CREATURE_SPELLS];
    let creature = wow_entities::Creature::create_from_lifecycle(CreatureCreateLifecycleRecord {
        guid: creature_guid,
        entry: 9001,
        map_id: 0,
        instance_id: 0,
        position: Position::new(10.0, 10.0, 0.0, 0.0),
        dynamic: false,
        vehicle_id: None,
        vehicle_kit_create_input: None,
        add_to_world_vehicle_reset_context: None,
        template: CreatureTemplateLifecycleRecord {
            entry: 9001,
            original_entry: 9001,
            difficulty_id: 0,
            name: "burning crusade guard".to_string(),
            ai_name: String::new(),
            script_name: String::new(),
            required_expansion: 1,
            trainer_class: 0,
            unit_class: 1,
            faction: 14,
            npc_flags: 0,
            display_id: 100,
            model_dimensions: None,
            scale: 1.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
            spells,
            classification: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            flags_extra: 0,
            static_flags: [0; 8],
            creature_type: 0,
            type_flags: 0,
            loot_id: 0,
            skin_loot_id: 0,
            gold_min: 0,
            gold_max: 0,
            movement_type: MovementGeneratorType::Idle,
            ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
            swim_allowed: true,
            flight_movement_type: 0,
            rooted: false,
            chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
            random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
            interaction_pause_timer_ms:
                wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
            min_level: 80,
            max_level: 80,
            equipment_id: 0,
            original_equipment_id: 0,
        },
        spawn: Some(CreatureSpawnLifecycleRecord {
            spawn_id: 91_132,
            map_id: 0,
            instance_id: 0,
            position: Position::new(10.0, 10.0, 0.0, 0.0),
            home_position: Position::new(10.0, 10.0, 0.0, 0.0),
            phase_id: None,
            phase_group: None,
            terrain_swap_map: None,
            spawn_group_id: None,
            spawn_group_name: None,
            pool_id: None,
            equipment_id: None,
            original_equipment_id: None,
            wander_distance: 0.0,
            respawn_delay: 30,
            respawn_time: 0,
            movement_type: MovementGeneratorType::Idle,
            string_id: None,
            is_active: true,
            inactive_by_spawn_group: false,
            duplicate_spawn_found: false,
            add_to_map: true,
            respawn_compatibility_mode: false,
        }),
        selected_level: 80,
        stats: CreatureLifecycleStats::new(100, 100, 0, 0),
        selected_display_id: 100,
        selected_model_dimensions: None,
        selected_equipment_id: 0,
        selected_original_equipment_id: 0,
        selected_virtual_items: [(0, 0, 0); 3],
        corpse_delay: 30,
        ignore_corpse_decay_ratio: false,
        addon: None,
    });
    let mut world_creature = WorldCreature::from_canonical(
        creature,
        test_creature_create_data(creature_guid, 9001, 100),
    );
    world_creature.creature.configure_ai_runtime(
        Position::new(10.0, 10.0, 0.0, 0.0),
        500.0,
        0.0,
        30,
    );
    world_creature
        .creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(9001);
    world_creature.creature.unit_mut().set_level(80);
    world_creature.creature.unit_mut().set_combat_reach(0.0);
    {
        let mut guard = manager.write().unwrap();
        guard.add_creature(0, 0, 0, 0, world_creature);
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    }

    let player = ObjectGuid::create_player(1, 91_133);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(27.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(
        outcome.aggro_starts, 0,
        "C++ Creature::GetAttackDistance caps level 80 creatures to level 70 when RequiredExpansion=1"
    );
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_adds_combat_distance_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_130);
    register_test_creature(&mut session, manager.clone(), creature_guid, 80);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 500.0;
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_combat_distance_like_cpp(2.0);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_131);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(31.5, 10.0, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.aggro_starts, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].victim_guid, player);
}
#[test]
fn legacy_creature_aggro_tick_once_rejects_targets_beyond_home_range_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_023);
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

    let player = ObjectGuid::create_player(1, 91_024);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(
            10.0 + crate::map_manager::VISIBILITY_RADIUS + 1.0,
            10.0,
            0.0,
            0.0,
        ),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_home_range_rejects_exact_3d_edge_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_060);
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

    let player = ObjectGuid::create_player(1, 91_061);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(
            10.0 + wow_entities::DEFAULT_VISIBILITY_DISTANCE,
            10.0,
            0.0,
            0.0,
        ),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_home_range_rejects_exact_2d_flight_edge_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_062);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 250.0;
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

    let player = ObjectGuid::create_player(1, 91_063);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(
            10.0 + wow_entities::DEFAULT_VISIBILITY_DISTANCE,
            10.0,
            250.0,
            0.0,
        ),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.home_range_rejections, 1);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
#[test]
fn legacy_creature_aggro_tick_once_skips_evading_attacker_as_sightless_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_046);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.ai_ownership_mut().aggro_radius = 50.0;
            creature.creature.unit_mut().set_level(25);
            creature.creature.set_in_evade_mode_like_cpp(true);
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let player = ObjectGuid::create_player(1, 91_047);
    let candidates = vec![legacy_aggro_candidate_like_cpp(
        player,
        Position::new(10.5, 10.5, 0.0, 0.0),
    )];

    let outcome = run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        &manager,
        &candidates,
        legacy_aggro_hostile_config_like_cpp(),
    );

    assert_eq!(outcome.sightless_creatures_skipped, 1);
    assert_eq!(outcome.attacker_evade_rejections, 0);
    assert_eq!(outcome.aggro_starts, 0);
    assert!(outcome.commands.is_empty());
}
