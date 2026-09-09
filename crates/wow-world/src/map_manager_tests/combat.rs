//! Combat scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn create_data_from_canonical_clamps_zero_base_attack_time_like_cpp() {
    // Regression for the world-entry client crash: a creature CREATE block with
    // UnitData.AttackRoundBaseTime == 0 makes the 3.4.3 client divide-by-zero in its
    // swing-timer math on the first post-spawn tick (crash ~5s after the visibility
    // burst). C++ guarantees this is never 0 (ObjectMgr.cpp:1100-1104 clamps
    // creature_template BaseAttackTime/RangeAttackTime 0 -> BASE_ATTACK_TIME=2000).
    // A bare canonical Creature leaves Unit::base_attack_speed at its [0; MAX_ATTACK]
    // default, reproducing the bug; create_data_from_canonical_like_cpp must clamp it.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9001);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9001);
    // Sanity: the underlying base attack speed really is the uninitialized 0 here.
    assert_eq!(
        creature.unit().base_attack_speed()[WeaponAttackType::BaseAttack as usize],
        0,
        "precondition: bare canonical creature has 0 base attack speed"
    );

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(
        create_data.base_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
        "0 base attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
    );
    assert_eq!(
        create_data.ranged_attack_time, BASE_ATTACK_TIME_LIKE_CPP,
        "0 ranged attack time must be clamped to BASE_ATTACK_TIME (2000), never shipped as 0"
    );
}
#[test]
fn create_data_from_canonical_preserves_nonzero_base_attack_time_like_cpp() {
    // The clamp must only replace 0; a real attack time must pass through unchanged.
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 1, 9002);
    let mut creature = Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature.unit_mut().world_mut().object_mut().set_entry(9002);
    creature
        .unit_mut()
        .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 1500);
    creature
        .unit_mut()
        .set_base_attack_time_like_cpp(WeaponAttackType::RangedAttack, 1800);

    let create_data = WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(create_data.base_attack_time, 1500);
    assert_eq!(create_data.ranged_attack_time, 1800);
}
#[test]
fn pending_respawn_preserves_combat_log_state_across_legacy_and_canonical_like_cpp() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 45);
    let seed = test_creature(guid);
    let mut canonical = seed.creature;
    canonical.set_spawn_id(45);
    canonical.unit_mut().set_class(Class::Hunter as u8);
    canonical.set_power_type(PowerType::Focus);
    canonical.unit_mut().set_max_power(PowerType::Focus, 100);
    canonical.unit_mut().set_power(PowerType::Focus, 37);
    let combat_log_stats = CreatureCombatLogStatsLikeCpp {
        attack_power: 111,
        ranged_attack_power: 222,
        spell_power: 333,
        armor: 444,
    };
    canonical.set_combat_log_stats_like_cpp(combat_log_stats);
    let mut loaded_grid = WorldCreature::from_loaded_grid_canonical_like_cpp(canonical, |_| None);

    assert!(
        loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    );
    loaded_grid
        .creature
        .set_death_state_runtime(DeathState::JustDied, 0);
    assert!(
        !loaded_grid
            .creature
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp(),
        "death cleanup must revoke the live marker before respawn"
    );

    let pending = pending_respawn_from_world_creature_like_cpp(&loaded_grid, Instant::now(), 0);
    assert_eq!(pending.create_data.unit_class, Class::Hunter as u8);
    assert_eq!(pending.create_data.display_power, PowerType::Focus as u8);
    assert_eq!(pending.create_data.power[0], 37);
    assert_eq!(pending.combat_log_stats, combat_log_stats);
    assert!(pending.spell_hit_aura_source_authority_like_cpp);
    assert!(pending.spell_cast_log_aura_source_authority_like_cpp);

    let legacy = world_creature_from_pending_respawn_like_cpp(&pending, 0);
    let canonical_mirror = legacy.creature.clone();
    for creature in [&legacy.creature, &canonical_mirror] {
        assert_eq!(creature.unit().data().class_id, Class::Hunter as u8);
        assert_eq!(creature.power_type(), PowerType::Focus);
        assert_eq!(creature.unit().get_power(PowerType::Focus), 37);
        assert_eq!(creature.combat_log_stats_like_cpp(), combat_log_stats);
        assert_eq!(creature.combat_log_attack_power_like_cpp(), 222);
        assert!(
            creature
                .unit()
                .subsystems()
                .auras
                .has_complete_spell_hit_inert_aura_authority_like_cpp()
        );
        assert!(
            creature
                .unit()
                .subsystems()
                .auras
                .has_complete_spell_cast_log_aura_authority_like_cpp()
        );
    }
}
