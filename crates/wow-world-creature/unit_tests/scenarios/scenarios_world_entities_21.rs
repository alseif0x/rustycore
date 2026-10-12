//! Creature runtime scenarios of the represented world entities responsibility.
//!
//! #1363 slice 3b moved these session-free tests verbatim out of the `wow-world`
//! session scenario module `scenarios_world_entities_21`; the tests there that
//! build a `WorldSession` stay in `wow-world`.

use super::*;

#[test]
fn creature_aggro_tick_once_uses_template_required_expansion_cap_like_cpp() {
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
    let mut world_creature = creature;
    crate::creature_projection::install_creature_runtime_like_cpp(
        &mut world_creature,
        &test_creature_create_data(creature_guid, 9001, 100),
    );
    world_creature.configure_ai_runtime(Position::new(10.0, 10.0, 0.0, 0.0), 500.0, 0.0, 30);
    world_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(9001);
    world_creature.unit_mut().set_level(80);
    world_creature.unit_mut().set_combat_reach(0.0);
    {
        let mut guard = manager.lock().unwrap();
        guard.add_creature(0, 0, world_creature);
    }

    let player = ObjectGuid::create_player(1, 91_133);
    let candidates = vec![aggro_candidate_like_cpp(
        player,
        Position::new(27.0, 10.0, 0.0, 0.0),
    )];

    let outcome = run_canonical_creature_aggro_phase_for_test_like_cpp(
        &manager,
        &candidates,
        aggro_hostile_config_like_cpp(),
    );

    assert_eq!(
        outcome.aggro_starts, 0,
        "C++ Creature::GetAttackDistance caps level 80 creatures to level 70 when RequiredExpansion=1"
    );
    assert!(outcome.commands.is_empty());
}
