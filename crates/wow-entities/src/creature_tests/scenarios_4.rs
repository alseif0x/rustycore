//! Creature regressions, part 4 of 4.
//!
//! Moved out of the creature_tests.rs root under #648; every test is unchanged.

use super::*;

#[test]
fn creature_runtime_just_respawned_resets_represented_runtime_state() {
    let player = ObjectGuid::new(1, 3);
    let mut creature = Creature::new(false);
    creature.set_ai_identity_runtime(
        100,
        35,
        0x40,
        (UnitFlags::IMMUNE_TO_NPC | UnitFlags::IN_COMBAT).bits(),
    );
    creature.set_npc_flags2_runtime_like_cpp(0x2);
    creature.set_unit_flags2_runtime_like_cpp(UnitFlags2::FEIGN_DEATH.bits());
    creature.set_unit_flags3_runtime_like_cpp(UnitFlags3::AI_OBSTACLE.bits());
    creature.unit_mut().set_max_health(250);
    creature.unit_mut().set_health(1);
    creature.unit_mut().set_death_state(DeathState::Corpse);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_dynamic_flag(UnitDynFlags::Lootable as u32);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_dynamic_flag(UnitDynFlags::CanSkin as u32);
    creature
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);
    creature
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::empty());
    creature
        .unit_mut()
        .set_unit_flags3_like_cpp(UnitFlags3::empty());
    creature.unit_mut().set_npc_flags_like_cpp(0);
    creature.unit_mut().set_npc_flags2_like_cpp(0);
    creature.unit_mut().add_unit_state(
        UnitState::DIED.bits()
            | UnitState::CHARGING.bits()
            | UnitState::ROAMING_MOVE.bits()
            | UnitState::IGNORE_PATHFINDING.bits(),
    );
    creature.player_damage_req = 42;
    creature.cannot_reach_target = true;
    creature.cannot_reach_timer = 900;
    creature.set_respawn_time(123);
    creature.corpse_remove_time = 99;
    creature.set_pickpocket_loot_restore(777);
    creature.loot_mode = 0x4;
    creature.set_tapped_by_player(player, &[]);
    creature.set_melee_damage_school_like_cpp(wow_constants::spell::SpellSchools::Fire as u8);

    let plan = creature.set_death_state_runtime(DeathState::JustRespawned, 5_000);

    assert_eq!(creature.unit().death_state(), DeathState::Alive);
    assert_eq!(creature.unit().data().health, 250);
    assert_eq!(
        creature.unit().world().object().dynamic_flags(),
        0,
        "C++ Creature::setDeathState(JUST_RESPAWNED) calls ReplaceAllDynamicFlags(UNIT_DYNFLAG_NONE)"
    );
    assert!(
        !creature
            .unit()
            .unit_flags_like_cpp()
            .intersects(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT),
        "C++ Unit::setDeathState(JUST_RESPAWNED) removes SKINNABLE and Creature respawn removes IN_COMBAT"
    );
    assert!(
        creature
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IMMUNE_TO_NPC),
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags via ChooseCreatureFlags"
    );
    assert_eq!(
        creature.unit().unit_flags2_like_cpp(),
        UnitFlags2::FEIGN_DEATH,
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags2"
    );
    assert_eq!(
        creature.unit().unit_flags3_like_cpp(),
        UnitFlags3::AI_OBSTACLE,
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads template unitFlags3"
    );
    assert_eq!(
        creature.unit().npc_flags_like_cpp(),
        [0x40, 0x2],
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads npcFlags/npcFlags2"
    );
    assert_eq!(
        creature.melee_damage_school_mask(),
        1 << (wow_constants::spell::SpellSchools::Normal as u8),
        "C++ Creature::setDeathState(JUST_RESPAWNED) reloads melee damage school from cInfo->dmgschool"
    );
    assert_eq!(
        UnitState::from_bits_truncate(creature.unit().unit_state()),
        UnitState::IGNORE_PATHFINDING,
        "C++ Creature::setDeathState(JUST_RESPAWNED) clears UNIT_STATE_ALL_ERASABLE but preserves IGNORE_PATHFINDING"
    );
    assert!(creature.tap_list().is_empty());
    assert_eq!(creature.player_damage_req(), 0);
    assert!(!creature.cannot_reach_target());
    assert_eq!(creature.cannot_reach_timer(), 0);
    assert_eq!(creature.respawn_time(), 0);
    assert_eq!(creature.corpse_remove_time(), 0);
    assert_eq!(creature.pickpocket_loot_restore(), 0);
    assert_eq!(creature.loot_mode(), LOOT_MODE_DEFAULT);
    assert!(creature.trigger_just_appeared());
    assert!(plan.contains(CreatureRuntimeAction::ClearTapList));
    assert!(plan.contains(CreatureRuntimeAction::ResetAi));
}

#[test]
fn creature_runtime_forced_despawn_immediate_matches_compat_and_noncompat_bridges() {
    let now = 20_000;
    let mut compat = Creature::new(false);
    compat.set_respawn_compatibility_mode(true);
    compat.set_respawn_delay(300);
    compat.set_corpse_delay(60, false);

    let plan = compat.forced_despawn_runtime(0, 42, now);

    assert_eq!(compat.unit().death_state(), DeathState::Dead);
    assert_eq!(compat.respawn_delay(), 300);
    assert_eq!(compat.corpse_delay(), 60);
    assert_eq!(compat.respawn_time(), now + 42);
    assert_eq!(compat.corpse_remove_time(), now);
    assert!(plan.contains(CreatureRuntimeAction::DestroyVisibility));
    assert!(plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));

    let mut delayed = Creature::new(false);
    let delayed_plan = delayed.forced_despawn_runtime(500, 0, now);
    assert!(delayed.runtime_state().forced_despawn_pending);
    assert!(delayed_plan.contains(CreatureRuntimeAction::RequestDelayedForcedDespawn));

    let mut non_compat = Creature::new(false);
    non_compat.set_respawn_compatibility_mode(false);
    non_compat.set_respawn_delay(55);
    let non_compat_plan = non_compat.forced_despawn_runtime(0, 0, now);
    assert_eq!(non_compat.respawn_time(), now + 55);
    assert!(non_compat.runtime_state().save_respawn_requested);
    assert!(non_compat.runtime_state().object_remove_requested);
    assert!(non_compat_plan.contains(CreatureRuntimeAction::SaveRespawnTime));
    assert!(non_compat_plan.contains(CreatureRuntimeAction::RequestObjectRemove));
}

#[test]
fn creature_corpse_removal_retires_arc_held_loot_authority() {
    let mut creature = Creature::new(false);
    creature.replace_loot_authority_like_cpp(None, HashMap::new());
    let authority = creature.loot_authority_like_cpp().clone();
    creature.unit_mut().set_death_state(DeathState::Corpse);
    assert!(!authority.is_retired_like_cpp());

    let plan = creature.remove_corpse_runtime(20_000, true, false);

    assert!(plan.contains(CreatureRuntimeAction::RemoveLoot));
    assert!(
        authority.is_retired_like_cpp(),
        "corpse removal must invalidate claims that outlive the map object"
    );
}

#[test]
fn creature_runtime_all_loot_removed_updates_corpse_and_respawn_like_trinity() {
    let now = 1_000;
    let mut creature = Creature::new(false);
    creature.set_corpse_delay(60, false);
    creature.set_respawn_delay(300);
    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(now + 100);

    let plan = creature.all_loot_removed_from_corpse(now, 0.5, false);

    assert_eq!(creature.corpse_remove_time(), now + 30);
    assert_eq!(creature.respawn_time(), now + 330);
    assert!(plan.contains(CreatureRuntimeAction::UpdateLoot));

    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(now + 1_000);
    creature.all_loot_removed_from_corpse(now, 0.5, true);
    assert_eq!(creature.corpse_remove_time(), now);
    assert_eq!(creature.respawn_time(), now + 1_000);

    creature.set_corpse_delay(60, true);
    creature.corpse_remove_time = now + 600;
    creature.set_respawn_time(0);
    creature.all_loot_removed_from_corpse(now, 0.01, false);
    assert_eq!(creature.corpse_remove_time(), now + 60);
}

#[test]
fn creature_runtime_tap_list_group_soft_cap_and_evade_clear_rules() {
    let player = ObjectGuid::new(1, 1);
    let group = [
        ObjectGuid::new(1, 2),
        ObjectGuid::new(1, 3),
        ObjectGuid::new(1, 4),
        ObjectGuid::new(1, 5),
        ObjectGuid::new(1, 6),
    ];
    let mut creature = Creature::new(false);

    creature.set_tapped_by_player(player, &group);

    assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
    assert!(creature.is_tapped_by(player));
    assert!(creature.is_tapped_by(group[0]));
    assert!(!creature.is_tapped_by(group[4]));
    assert!(creature.has_loot_recipient());

    creature.set_dont_clear_tap_list_on_evade(true);
    assert!(creature.dont_clear_tap_list_on_evade());
    creature.clear_tap_list_for_evade();
    assert_eq!(creature.tap_list().len(), CREATURE_TAPPERS_SOFT_CAP);
    creature.clear_tap_list();
    assert!(creature.tap_list().is_empty());

    let mut spawned_creature = Creature::new(false);
    spawned_creature.set_spawn_id(99);
    spawned_creature.set_dont_clear_tap_list_on_evade(true);
    assert!(!spawned_creature.dont_clear_tap_list_on_evade());
}

#[test]
fn creature_evading_attacks_matches_cpp_evade_or_cannot_reach() {
    let mut creature = Creature::new(false);

    assert!(!creature.is_in_evade_mode_like_cpp());
    assert!(!creature.is_evading_attacks_like_cpp());

    creature.set_in_evade_mode_like_cpp(true);
    assert!(creature.is_in_evade_mode_like_cpp());
    assert!(creature.is_evading_attacks_like_cpp());

    creature.set_in_evade_mode_like_cpp(false);
    assert!(!creature.is_evading_attacks_like_cpp());

    creature.set_cannot_reach_target_like_cpp(true);
    assert!(creature.cannot_reach_target());
    assert!(creature.is_evading_attacks_like_cpp());

    creature.cannot_reach_timer = 500;
    creature.set_cannot_reach_target_like_cpp(false);
    assert!(!creature.cannot_reach_target());
    assert_eq!(creature.cannot_reach_timer(), 0);
    assert!(!creature.is_evading_attacks_like_cpp());
}

#[test]
fn creature_lifecycle_init_entry_derives_static_flags_like_cpp() {
    let mut record = creature_lifecycle_create_record();
    record.vehicle_id = None;
    record.vehicle_kit_create_input = None;
    record.template.rooted = false;
    record.template.flags_extra |= CreatureFlagsExtra::NO_XP.bits();
    record.template.static_flags[0] =
        CreatureStaticFlags::SESSILE.bits() | CreatureStaticFlags::NO_MELEE_FLEE.bits();
    record.template.type_flags |= CreatureTypeFlags::TREAT_AS_RAID_UNIT.bits();

    let creature = Creature::create_from_lifecycle(record);
    let primary =
        CreatureStaticFlags::from_bits_truncate(creature.lifecycle_metadata().static_flags[0]);
    let flags4 =
        CreatureStaticFlags4::from_bits_truncate(creature.lifecycle_metadata().static_flags[3]);

    assert!(primary.contains(CreatureStaticFlags::SESSILE));
    assert!(primary.contains(CreatureStaticFlags::NO_MELEE_FLEE));
    assert!(primary.contains(CreatureStaticFlags::NO_XP));
    assert!(!creature.can_give_experience_like_cpp());
    assert!(flags4.contains(CreatureStaticFlags4::TREAT_AS_RAID_UNIT_FOR_HELPFUL_SPELLS));
    assert!(creature.is_template_rooted_like_cpp());
    assert!(creature.unit().has_unit_state(UnitState::ROOT.bits()));
    assert!(
        creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::ROOT)
    );
    assert!(!creature.can_melee_like_cpp());
}

#[test]
fn creature_can_melee_reflects_primary_static_no_melee_flag_like_cpp() {
    let mut creature = Creature::new(false);
    assert!(creature.can_melee_like_cpp());

    let mut static_flags = [0; 8];
    static_flags[0] = CreatureStaticFlags::NO_MELEE_FLEE.bits();
    creature.set_static_flags_runtime_like_cpp(static_flags);

    assert!(!creature.can_melee_like_cpp());
}

#[test]
fn creature_runtime_update_plan_covers_dead_corpse_and_alive_branches() {
    let now = 50_000;
    let mut dead = Creature::new(false);
    dead.set_respawn_compatibility_mode(true);
    dead.set_respawn_time(now);
    dead.unit_mut().set_death_state(DeathState::Dead);
    let dead_plan = dead.runtime_update_plan(1, now, CreatureRuntimeUpdateContext::default());
    assert!(dead_plan.contains(CreatureRuntimeAction::ResetAi));
    assert_eq!(dead.unit().death_state(), DeathState::Alive);

    let mut corpse = Creature::new(false);
    corpse.set_respawn_compatibility_mode(true);
    corpse.unit_mut().set_death_state(DeathState::Corpse);
    corpse.corpse_remove_time = now;
    let corpse_plan = corpse.runtime_update_plan(
        1,
        now,
        CreatureRuntimeUpdateContext {
            has_loot: true,
            ..CreatureRuntimeUpdateContext::default()
        },
    );
    assert!(corpse_plan.contains(CreatureRuntimeAction::UpdateLoot));
    assert!(corpse_plan.contains(CreatureRuntimeAction::RelocateToRespawnPosition));
    assert_eq!(corpse.unit().death_state(), DeathState::Dead);

    let mut alive = Creature::new(false);
    alive.boundary_check_time = 10;
    alive.combat_pulse_delay = 2;
    alive.combat_pulse_time = 1;
    alive.regen_timer = 1;
    alive.cannot_reach_timer = CREATURE_NOPATH_EVADE_TIME_MS - 5;
    let alive_plan = alive.runtime_update_plan(
        10,
        now,
        CreatureRuntimeUpdateContext {
            is_engaged: true,
            is_dungeon: true,
            has_map_players: true,
            cannot_reach_target: true,
            ..CreatureRuntimeUpdateContext::default()
        },
    );
    assert!(alive_plan.contains(CreatureRuntimeAction::NotifyJustAppeared));
    assert!(alive_plan.contains(CreatureRuntimeAction::BoundaryCheck));
    assert!(alive_plan.contains(CreatureRuntimeAction::CombatPulse));
    assert!(alive_plan.contains(CreatureRuntimeAction::RegeneratePower));
    assert!(alive_plan.contains(CreatureRuntimeAction::Evade(
        CreatureRuntimeEvadeReason::NoPath
    )));
}
