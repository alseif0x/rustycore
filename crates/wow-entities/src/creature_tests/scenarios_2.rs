//! Creature regressions, part 2 of 4.
//!
//! Moved out of the creature_tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn creature_react_state_and_faction_use_unit_fields() {
    let mut creature = Creature::new(false);

    creature.set_react_state(ReactState::Passive);
    creature.set_faction(35);

    assert!(creature.has_react_state(ReactState::Passive));
    assert_eq!(creature.unit().data().faction_template, 35);
}

#[test]
fn creature_grid_unload_helpers_apply_represented_state() {
    let victim = wow_core::ObjectGuid::new(1, 2);
    let dynamic_object = wow_core::ObjectGuid::new(1, 3);
    let area_trigger = wow_core::ObjectGuid::new(1, 4);
    let mut creature = Creature::new(false);
    creature.unit_mut().set_attacking(Some(victim));
    creature.unit_mut().world_mut().set_current_cell(7, 8);
    creature.register_dynamic_object(dynamic_object);
    creature.register_area_trigger(area_trigger);

    creature.set_destroyed_object(true);
    creature.remove_all_dyn_objects();
    creature.remove_all_area_triggers();
    creature.combat_stop();
    creature.request_respawn_relocation_from_grid_unload();
    creature.cleanup_before_delete();
    creature.request_delete_from_grid_unload();

    assert!(creature.unit().world().object().is_destroyed_object());
    assert!(creature.dynamic_objects().is_empty());
    assert_eq!(
        creature.removed_dynamic_objects_from_grid_unload(),
        &[dynamic_object]
    );
    assert!(creature.area_triggers().is_empty());
    assert_eq!(
        creature.removed_area_triggers_from_grid_unload(),
        &[area_trigger]
    );
    assert_eq!(creature.unit().attacking(), None);
    assert!(creature.grid_unload_respawn_relocation_requested());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
    assert!(creature.grid_unload_delete_requested());
    assert_eq!(creature.unit().world().current_cell(), None);
    assert!(!creature.unit().world().object().is_in_grid());
}

#[test]
fn creature_lifecycle_create_sets_ignore_pathfinding_from_flags_extra_like_cpp() {
    // C++ `Creature::Create` (`Creature.cpp:1154-1155`).
    let baseline = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    assert!(
        !baseline
            .unit()
            .has_unit_state(UnitState::IGNORE_PATHFINDING.bits()),
        "a template without the flag must keep using the navmesh"
    );

    let mut record = creature_lifecycle_create_record();
    record.template.flags_extra |= CreatureFlagsExtra::IGNORE_PATHFINDING.bits();
    let ignoring = Creature::create_from_lifecycle(record);
    assert!(
        ignoring
            .unit()
            .has_unit_state(UnitState::IGNORE_PATHFINDING.bits()),
        "CREATURE_FLAG_EXTRA_IGNORE_PATHFINDING must add UNIT_STATE_IGNORE_PATHFINDING"
    );
    assert_eq!(
        CreatureFlagsExtra::IGNORE_PATHFINDING.bits(),
        0x2000_0000,
        "flag value must match CreatureData.h:363"
    );
}

#[test]
fn creature_lifecycle_create_applies_template_stats_and_clean_baseline() {
    let creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());

    assert_eq!(
        creature.unit().world().object().guid(),
        ObjectGuid::new(8, 1001)
    );
    assert_eq!(creature.unit().world().object().entry(), 1001);
    assert_eq!(creature.unit().world().map_id(), 571);
    assert_eq!(creature.unit().world().instance_id(), 3);
    assert_eq!(
        creature.unit().world().position(),
        Position::new(1.0, 2.0, 3.0, 4.0)
    );
    assert_eq!(creature.unit().data().race, 0);
    assert_eq!(creature.unit().data().class_id, 1);
    assert_eq!(creature.unit().data().faction_template, 14);
    assert_eq!(creature.unit().npc_flags_like_cpp(), [0x40, 0x1]);
    assert_eq!(
        creature.unit().unit_flags_like_cpp(),
        UnitFlags::IMMUNE_TO_NPC
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE
    );
    assert_eq!(creature.unit().data().display_id, 3001);
    assert_eq!(creature.unit().data().native_display_id, 3001);
    assert_eq!(creature.unit().world().object().scale(), 1.5);
    assert_eq!(
        creature.unit().data().bounding_radius,
        0.5 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
    );
    assert_eq!(
        creature.unit().data().combat_reach,
        2.0 * 1.5 * crate::DEFAULT_PLAYER_DISPLAY_SCALE
    );
    let speed_rate = creature.unit().speed_rate();
    assert_eq!(speed_rate[UnitMoveType::Walk as usize], 0.8);
    assert_eq!(speed_rate[UnitMoveType::Run as usize], 1.25);
    assert_eq!(speed_rate[UnitMoveType::Swim as usize], 1.0);
    assert_eq!(speed_rate[UnitMoveType::Flight as usize], 1.0);
    assert_eq!(creature.unit().data().mod_casting_speed, 1.0);
    assert_eq!(creature.unit().data().mod_spell_haste, 1.0);
    assert_eq!(creature.unit().data().mod_haste, 1.0);
    assert_eq!(creature.unit().data().mod_ranged_haste, 1.0);
    assert_eq!(creature.unit().data().mod_haste_regen, 1.0);
    assert_eq!(creature.unit().data().mod_time_rate, 1.0);
    assert!(creature.unit().can_dual_wield_like_cpp());
    assert_eq!(creature.unit().data().virtual_items[0].item_id, 10_001);
    assert_eq!(
        creature.unit().data().virtual_items[0].item_appearance_mod_id,
        3
    );
    assert_eq!(creature.unit().data().virtual_items[0].item_visual, 4);
    assert_eq!(creature.unit().data().virtual_items[1].item_id, 10_002);
    assert_eq!(
        creature.unit().data().virtual_items[1].item_appearance_mod_id,
        5
    );
    assert_eq!(creature.unit().data().virtual_items[1].item_visual, 6);
    assert_eq!(
        creature.unit().data().virtual_items[2],
        VisibleItemValues::default()
    );
    assert_eq!(creature.spells()[0], 133);
    assert_eq!(creature.spells()[3], 116);
    assert_eq!(
        creature.melee_damage_school_mask(),
        1 << (wow_constants::spell::SpellSchools::Nature as u8),
        "C++ Creature::UpdateEntry applies SetMeleeDamageSchool(cInfo->dmgschool)"
    );
    assert_eq!(creature.equipment_id(), 6);
    assert_eq!(creature.original_equipment_id(), -6);
    let kit = creature.unit().subsystems().vehicle.kit.as_ref().unwrap();
    assert_eq!(kit.kit_id(), 101);
    assert!(kit.active());
    assert!(!kit.installed());
    assert_eq!(kit.seat_count(), 2);
    assert_eq!(kit.usable_seat_num(), 1);
    let create_outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .unwrap();
    assert_eq!(create_outcome.kit_id, Some(101));
    assert!(create_outcome.created);
    assert_eq!(create_outcome.seat_count, 2);
    assert_eq!(create_outcome.usable_seat_num, 1);
    assert!(create_outcome.unit_update_flag_vehicle_represented);
    assert!(create_outcome.unit_type_mask_vehicle_represented);
    assert!(creature.has_unit_type_mask_like_cpp(UNIT_MASK_VEHICLE));
    assert!(creature.is_vehicle_unit_type_like_cpp());
    assert!(!creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(!create_outcome.send_set_vehicle_rec_id_represented);
    assert!(create_outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!create_outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(create_outcome.update_display_power_represented);
    assert!(create_outcome.init_movement_info_for_base_represented);
    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(101));
    assert_eq!(creature.unit().data().level, 71);
    assert_eq!(creature.unit().data().max_health, 5_000);
    assert_eq!(creature.unit().data().health, 4_500);
    assert_eq!(creature.unit().get_max_power(PowerType::Mana), 1_000);
    assert_eq!(creature.unit().get_power(PowerType::Mana), 750);
    assert_eq!(creature.lifecycle_metadata().spawn_health, Some(4_500));
    assert_eq!(creature.lifecycle_metadata().spawn_mana, Some(750));
    assert_eq!(
        creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
        [BASE_MINDAMAGE, BASE_MAXDAMAGE]
    );
    assert_eq!(creature.corpse_delay(), 90);
    assert!(creature.ignore_corpse_decay_ratio());
    assert!(creature.respawn_compatibility_mode());
    assert_eq!(creature.lifecycle_metadata().template_entry, 1001);
    assert_eq!(creature.lifecycle_metadata().original_entry, 9001);
    assert_eq!(creature.lifecycle_metadata().difficulty_id, 2);
    assert_eq!(creature.lifecycle_metadata().ai_name, "SmartAI");
    assert_eq!(
        creature.lifecycle_metadata().script_name,
        "npc_lifecycle_wolf"
    );
    assert_eq!(creature.lifecycle_metadata().required_expansion, 2);
    assert_eq!(creature.lifecycle_metadata().classification, 3);
    assert_eq!(
        creature.lifecycle_metadata().damage_school,
        wow_constants::spell::SpellSchools::Nature as u8
    );
    assert_eq!(
        creature.lifecycle_metadata().flight_movement_type,
        CreatureFlightMovementType::DisableGravity as u8
    );
    assert_eq!(creature.unit().changed_object_type_mask(), 0);
}

#[test]
fn creature_lifecycle_retains_combat_log_stats_and_selects_attack_power_like_cpp() {
    let combat_log_stats = CreatureCombatLogStatsLikeCpp {
        attack_power: 111,
        ranged_attack_power: 222,
        spell_power: 333,
        armor: 444,
    };

    let mut melee_record = creature_lifecycle_create_record();
    melee_record.stats.combat_log = combat_log_stats;
    let melee = Creature::create_from_lifecycle(melee_record);

    assert_eq!(melee.combat_log_stats_like_cpp(), combat_log_stats);
    assert_eq!(melee.combat_log_attack_power_like_cpp(), 111);

    let mut hunter_record = creature_lifecycle_create_record();
    hunter_record.template.unit_class = Class::Hunter as u8;
    hunter_record.stats.combat_log = combat_log_stats;
    let hunter = Creature::create_from_lifecycle(hunter_record);

    assert_eq!(hunter.combat_log_stats_like_cpp(), combat_log_stats);
    assert_eq!(hunter.combat_log_attack_power_like_cpp(), 222);
}

#[test]
fn creature_lifecycle_seeds_selected_non_mana_power_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.stats = CreatureLifecycleStats {
        max_health: 5_000,
        health: 4_500,
        power_type: PowerType::Focus,
        base_mana: 600,
        max_power: 100,
        power: 25,
        min_damage: BASE_MINDAMAGE,
        max_damage: BASE_MAXDAMAGE,
        combat_log: CreatureCombatLogStatsLikeCpp::default(),
    };

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.power_type(), PowerType::Focus);
    assert_eq!(creature.unit().get_create_mana_like_cpp(), 600);
    assert_eq!(creature.unit().get_max_power(PowerType::Focus), 100);
    assert_eq!(creature.unit().get_power(PowerType::Focus), 25);
    assert_eq!(
        creature.unit().get_max_power(PowerType::Mana),
        0,
        "changing display power removes POWER_MANA from the creature power index"
    );
}

#[test]
fn creature_lifecycle_create_without_spawn_applies_dynamic_respawn_compatibility() {
    let mut record = creature_lifecycle_create_record();
    record.dynamic = false;
    record.spawn = None;
    let static_creature = Creature::create_from_lifecycle(record);
    assert!(static_creature.respawn_compatibility_mode());

    let mut record = creature_lifecycle_create_record();
    record.dynamic = true;
    record.spawn = None;
    let dynamic_creature = Creature::create_from_lifecycle(record);
    assert!(!dynamic_creature.respawn_compatibility_mode());
}

#[test]
fn creature_runtime_just_respawned_uses_represented_spawn_health_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_power(PowerType::Mana, 1);
    creature.unit_mut().set_death_state(DeathState::Corpse);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature.unit().data().health,
        4_500,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls SetSpawnHealth instead of always SetFullHealth for non-pets"
    );
    assert_eq!(
        creature.unit().get_power(PowerType::Mana),
        750,
        "C++ SetSpawnHealth restores the represented spawn mana source"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0x40, 0x1],
        "C++ JUST_RESPAWNED reloads ChooseCreatureFlags output from the creature template baseline"
    );
    assert_eq!(
        creature.unit().unit_flags_like_cpp(),
        UnitFlags::IMMUNE_TO_NPC
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE
    );
}

#[test]
fn creature_runtime_just_respawned_pet_uses_full_health_and_skips_non_pet_resets_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::new((HighGuid::Pet as i64) << 58, 44));
    creature.unit_mut().set_max_health(9_000);
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_power(PowerType::Mana, 1);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .replace_all_dynamic_flags(0x44);
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
    creature
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::FEIGN_DEATH);
    creature
        .unit_mut()
        .set_unit_flags3_like_cpp(UnitFlags3::AI_OBSTACLE);
    creature.set_melee_damage_school_like_cpp(wow_constants::spell::SpellSchools::Fire as u8);
    creature.unit_mut().set_death_state(DeathState::Corpse);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature.unit().data().health,
        9_000,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls SetFullHealth for pets"
    );
    assert_eq!(
        creature.unit().get_power(PowerType::Mana),
        1,
        "C++ pet branch does not run the non-pet spawn mana restore"
    );
    assert_eq!(
        creature.unit().world().object().dynamic_flags(),
        0x44,
        "C++ non-pet block owns ReplaceAllDynamicFlags(UNIT_DYNFLAG_NONE)"
    );
    assert!(
        creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT),
        "C++ non-pet block owns unit flag reload/removal"
    );
    assert_eq!(
        creature.melee_damage_school_like_cpp(),
        wow_constants::spell::SpellSchools::Fire as u8,
        "C++ non-pet block owns SetMeleeDamageSchool"
    );
    assert_eq!(creature.unit().death_state(), DeathState::Alive);
}

#[test]
fn creature_runtime_just_respawned_initializes_motion_when_not_blocked_by_formation_like_cpp() {
    let mut creature = Creature::create_from_lifecycle(creature_lifecycle_create_record());
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .set_current_generator(MovementGeneratorKind::Chase);
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase
    );

    let plan = creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert!(plan.contains(CreatureRuntimeAction::InitializeMotion));
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Idle,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls Motion_Initialize, which falls through to MotionMaster::Initialize when formation does not block it"
    );
}

#[test]
fn creature_runtime_just_respawned_preserves_non_leader_formation_motion_until_group_runtime_like_cpp()
 {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let mut spawn = creature_lifecycle_spawn();
    spawn.spawn_id = 44_001;
    let mut creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });
    creature.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: 44_000,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));
    creature
        .unit_mut()
        .subsystems_mut()
        .motion
        .set_current_generator(MovementGeneratorKind::Chase);

    creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Chase,
        "C++ non-leader formed creatures wait for CreatureGroup state; Rust has no live CreatureGroup::IsFormed runtime here yet"
    );
}

#[test]
fn aim_initialize_like_cpp_represents_normal_creature_without_formation_or_vehicle() {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let creature = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create,
        spawn: creature_lifecycle_spawn(),
    });

    let outcome = creature.aim_initialize_like_cpp();

    assert_eq!(outcome.guid, creature.guid());
    assert_eq!(outcome.spawn_id, 44_000);
    assert!(outcome.aim_create_represented);
    assert!(outcome.motion_initialize_represented);
    assert!(!outcome.formation_present);
    assert!(!outcome.formation_leader);
    assert!(!outcome.formation_move_idle_represented);
    assert!(!outcome.motion_initialize_requires_formed_state);
    assert!(outcome.motion_master_initialize_represented);
    assert!(outcome.ai_selected_represented);
    assert!(outcome.ai_initialize_represented);
    assert!(!outcome.vehicle_reset_expected);
    assert!(outcome.succeeded);
}

#[test]
fn aim_initialize_like_cpp_reports_formation_leader_and_non_leader_without_move_idle() {
    let mut create = creature_lifecycle_create_record();
    create.vehicle_id = None;
    create.vehicle_kit_create_input = None;
    let spawn = creature_lifecycle_spawn();
    let mut leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create: create.clone(),
        spawn: spawn.clone(),
    });
    leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: spawn.spawn_id,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));

    let leader_outcome = leader.aim_initialize_like_cpp();
    assert!(leader_outcome.formation_present);
    assert!(leader_outcome.formation_leader);
    assert!(!leader_outcome.formation_move_idle_represented);
    assert!(!leader_outcome.motion_initialize_requires_formed_state);
    assert!(leader_outcome.motion_master_initialize_represented);

    let mut non_leader_spawn = spawn;
    non_leader_spawn.spawn_id = 44_001;
    let mut non_leader = Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord {
        create,
        spawn: non_leader_spawn,
    });
    non_leader.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: 44_000,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));

    let non_leader_outcome = non_leader.aim_initialize_like_cpp();
    assert!(non_leader_outcome.formation_present);
    assert!(!non_leader_outcome.formation_leader);
    assert!(!non_leader_outcome.formation_move_idle_represented);
    assert!(non_leader_outcome.motion_initialize_requires_formed_state);
    assert!(!non_leader_outcome.motion_master_initialize_represented);
}

#[test]
fn creature_lifecycle_vehicle_entry_missing_preserves_identity_without_local_kit_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.vehicle_id = Some(909);
    record.vehicle_kit_create_input = None;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(creature.lifecycle_metadata().vehicle_id, Some(909));
    assert!(creature.unit().subsystems().vehicle.kit.is_none());
    let outcome = creature
        .unit()
        .subsystems()
        .vehicle
        .last_create_outcome
        .as_ref()
        .unwrap();
    assert_eq!(outcome.kit_id, Some(909));
    assert!(!outcome.created);
    assert_eq!(outcome.seat_count, 0);
    assert_eq!(outcome.usable_seat_num, 0);
    assert!(!outcome.unit_update_flag_vehicle_represented);
    assert!(!outcome.unit_type_mask_vehicle_represented);
    assert_eq!(creature.unit_type_mask_like_cpp(), 0);
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(!outcome.send_set_vehicle_rec_id_represented);
    assert!(!outcome.set_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!outcome.remove_spellclick_or_player_vehicle_npc_flag_represented);
    assert!(!outcome.update_display_power_represented);
    assert!(!outcome.init_movement_info_for_base_represented);
}

#[test]
fn creature_unit_type_mask_helpers_match_cpp_bitmask_semantics() {
    let mut creature = Creature::new(false);

    assert_eq!(creature.unit_type_mask_like_cpp(), 0);
    assert!(!creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(!creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(creature.can_have_threat_list_like_cpp());
    assert!(
        creature
            .unit()
            .subsystems()
            .combat
            .owner_can_have_threat_list
    );

    creature.add_unit_type_mask_like_cpp(
        UNIT_MASK_TOTEM | UNIT_MASK_GUARDIAN | UNIT_MASK_CONTROLABLE_GUARDIAN,
    );
    assert!(creature.has_unit_type_mask_like_cpp(UNIT_MASK_TOTEM));
    assert!(creature.is_totem_unit_type_like_cpp());
    assert!(creature.is_guardian_unit_type_like_cpp());
    assert!(creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.is_vehicle_unit_type_like_cpp());
    assert!(!creature.can_have_threat_list_like_cpp());
    assert!(
        !creature
            .unit()
            .subsystems()
            .combat
            .owner_can_have_threat_list
    );

    creature.remove_unit_type_mask_like_cpp(UNIT_MASK_GUARDIAN);
    assert!(creature.is_totem_unit_type_like_cpp());
    assert!(!creature.is_guardian_unit_type_like_cpp());
    assert!(creature.is_controlable_guardian_unit_type_like_cpp());
    assert!(!creature.can_have_threat_list_like_cpp());
}

#[test]
fn creature_lifecycle_create_applies_resolved_base_weapon_damage() {
    let mut record = creature_lifecycle_create_record();
    record.stats.min_damage = 3.5;
    record.stats.max_damage = 7.25;

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(
        creature.unit().weapon_damage(WeaponAttackType::BaseAttack),
        [3.5, 7.25]
    );
}

#[test]
fn creature_lifecycle_load_from_db_applies_spawn_bridge_state() {
    let create = creature_lifecycle_create_record();
    let spawn = creature_lifecycle_spawn();
    let creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });

    assert_eq!(creature.spawn_id(), 44_000);
    assert_eq!(creature.wander_distance(), 12.5);
    assert_eq!(creature.respawn_delay(), 45);
    assert_eq!(creature.respawn_time(), 123_456);
    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Idle
    );
    assert_eq!(creature.unit().world().map_id(), 571);
    assert_eq!(creature.unit().world().instance_id(), 3);
    assert_eq!(
        creature.unit().world().position(),
        Position::new(1.0, 2.0, 3.0, 4.0)
    );
    assert!(creature.respawn_compatibility_mode());
    assert_eq!(creature.equipment_id(), 9);
    assert_eq!(creature.original_equipment_id(), -9);
    let metadata = creature.lifecycle_metadata();
    assert_eq!(metadata.home_position, Position::new(5.0, 6.0, 7.0, 1.0));
    assert_eq!(metadata.phase_id, Some(169));
    assert_eq!(metadata.terrain_swap_map, Some(609));
    assert_eq!(metadata.spawn_group_id, Some(77));
    assert_eq!(
        metadata.spawn_group_name.as_deref(),
        Some("lifecycle group")
    );
    assert_eq!(metadata.string_id.as_deref(), Some("creature-string"));
    assert!(metadata.add_to_map_requested);
    assert!(metadata.map_insertion_requested);
    assert!(metadata.duplicate_spawn_found);
    assert!(!metadata.is_spawn_active);
    assert!(metadata.inactive_by_spawn_group);
    assert_eq!(creature.unit().changed_object_type_mask(), 0);
}

#[test]
fn creature_lifecycle_waypoint_default_survives_add_to_world_motion_initialize_like_cpp() {
    let create = creature_lifecycle_create_record();
    let mut spawn = creature_lifecycle_spawn();
    spawn.movement_type = MovementGeneratorType::Waypoint;
    let mut creature =
        Creature::load_from_db_lifecycle(CreatureLoadFromDbLifecycleRecord { create, spawn });

    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Waypoint
    );

    let outcome = creature.unit_mut().add_to_world_like_cpp();

    assert!(
        outcome
            .motion_master_add_to_world
            .had_initialization_pending
    );
    let current = creature
        .unit()
        .subsystems()
        .motion
        .current_movement_generator();
    assert_eq!(
        current.kind,
        MovementGeneratorKind::Waypoint,
        "C++ FactorySelector::SelectMovementGenerator uses Creature::GetDefaultMovementType during MotionMaster::InitializeDefault"
    );
    assert!(current.has_flag(crate::MOVEMENTGENERATOR_FLAG_INITIALIZATION_PENDING));
    assert_eq!(current.base_unit_state, UnitState::ROAMING.bits());
}

#[test]
fn creature_runtime_default_movement_setter_syncs_motion_generator_like_cpp() {
    let mut creature = Creature::new(false);
    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Idle
    );

    creature.set_default_movement_type_runtime_like_cpp(MovementGeneratorType::Waypoint);

    assert_eq!(
        creature.default_movement_type(),
        MovementGeneratorType::Waypoint
    );
    assert_eq!(
        creature
            .unit()
            .subsystems()
            .motion
            .current_movement_generator()
            .kind,
        MovementGeneratorKind::Waypoint
    );
}

#[test]
fn creature_lifecycle_loads_represented_addon_local_fields_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp {
        path_id: 9_001,
        mount_display_id: 12_345,
        stand_state: UnitStandStateType::Kneel,
        vis_flags: 0x12,
        anim_tier: 2,
        sheath_state: SheathState::Ranged,
        pvp_flags: UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP,
        emote: 77,
        ai_anim_kit_id: 11,
        movement_anim_kit_id: 22,
        melee_anim_kit_id: 33,
        visibility_distance_type: VisibilityDistanceTypeLikeCpp::Gigantic,
        auras: vec![70_001, 70_002],
        aura_applications: Vec::new(),
    });

    let creature = Creature::create_from_lifecycle(record);

    assert_eq!(
        creature.unit().data().mount_display_id,
        12_345,
        "C++ Creature::LoadCreaturesAddon calls Mount(addon->mount) when mount != 0"
    );
    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_001,
        "C++ Creature::LoadCreaturesAddon copies nonzero addon PathId into _waypointPathId"
    );
    assert_eq!(
        creature.unit().stand_state_like_cpp(),
        UnitStandStateType::Kneel,
        "C++ Creature::LoadCreaturesAddon calls SetStandState(addon->standState)"
    );
    assert_eq!(
        creature.unit().vis_flags_like_cpp(),
        0x12,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllVisFlags(addon->visFlags)"
    );
    assert_eq!(
        creature.unit().anim_tier_like_cpp(),
        2,
        "C++ Creature::LoadCreaturesAddon calls SetAnimTier(addon->animTier, false)"
    );
    assert_eq!(
        creature.unit().sheath_like_cpp(),
        SheathState::Ranged,
        "C++ Creature::LoadCreaturesAddon calls SetSheath(addon->sheathState)"
    );
    assert_eq!(
        creature.unit().pet_flags_like_cpp(),
        0,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllPetFlags(UNIT_PET_FLAG_NONE)"
    );
    assert_eq!(
        creature.unit().shapeshift_form_like_cpp(),
        ShapeShiftForm::None,
        "C++ Creature::LoadCreaturesAddon calls SetShapeshiftForm(FORM_NONE)"
    );
    assert_eq!(
        creature.unit().pvp_flags_like_cpp(),
        UnitPvpFlags::PVP | UnitPvpFlags::FFA_PVP,
        "C++ Creature::LoadCreaturesAddon calls ReplaceAllPvpFlags(addon->pvpFlags)"
    );
    assert_eq!(
        creature.unit().emote_state_like_cpp(),
        77,
        "C++ Creature::LoadCreaturesAddon calls SetEmoteState(addon->emote) when emote != 0"
    );
    assert_eq!(
        creature.unit().ai_anim_kit_id_like_cpp(),
        11,
        "C++ Creature::LoadCreaturesAddon calls SetAIAnimKitId(addon->aiAnimKit)"
    );
    assert_eq!(
        creature.unit().movement_anim_kit_id_like_cpp(),
        22,
        "C++ Creature::LoadCreaturesAddon calls SetMovementAnimKitId(addon->movementAnimKit)"
    );
    assert_eq!(
        creature.unit().melee_anim_kit_id_like_cpp(),
        33,
        "C++ Creature::LoadCreaturesAddon calls SetMeleeAnimKitId(addon->meleeAnimKit)"
    );
    assert_eq!(
        creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp(),
        Some(VisibilityDistanceTypeLikeCpp::Gigantic.distance_like_cpp()),
        "C++ Creature::LoadCreaturesAddon calls SetVisibilityDistanceOverride for non-Normal addon visibility"
    );
    assert!(
        creature
            .unit()
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::GIGANTIC_AOI),
        "C++ SetVisibilityDistanceOverride sets the matching UNIT_FLAG2_*_AOI flag"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_001),
        "C++ Creature::LoadCreaturesAddon applies listed permanent addon auras"
    );
    assert!(
        creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_002),
        "C++ Creature::LoadCreaturesAddon applies each listed addon aura"
    );
}

#[test]
fn creature_lifecycle_addon_applies_hover_movement_flag_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.template.ground_movement_type = CreatureGroundMovementType::Hover as u8;
    record.addon = Some(CreatureAddonLifecycleRecordLikeCpp::default());

    let creature = Creature::create_from_lifecycle(record);

    assert!(creature.can_hover_like_cpp());
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::HOVER),
        "C++ Creature::LoadCreaturesAddon calls AddUnitMovementFlag(MOVEMENTFLAG_HOVER) when CanHover()"
    );
}

#[test]
fn creature_load_path_sets_waypoint_path_id_like_cpp() {
    let mut creature = Creature::new(false);

    creature.load_path_like_cpp(9_123);

    assert_eq!(
        creature.waypoint_path_id_like_cpp(),
        9_123,
        "C++ Creature::LoadPath stores the waypoint path id used by WaypointMovementGenerator::DoInitialize"
    );
}
