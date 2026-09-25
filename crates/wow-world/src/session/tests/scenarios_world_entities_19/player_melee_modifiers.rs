//! Map-owned player-melee modifier scenarios.

use super::*;

/// The map-owned melee phase applies the victim's aurastate and aura-mechanic
/// melee bonuses, not only the session path.
///
/// `Unit::MeleeDamageBonusDone` reads the victim's `UNIT_FIELD_AURASTATE` and
/// `Unit::HasAuraWithMechanic` per swing (`Unit.cpp:7633-7648`). The production
/// swing owner is the map-owned runtime, so the masks must be resolved there
/// from the same receiver-free rule; a session-only implementation would let the
/// two owners diverge. The victim's health-derived wounded bit and an applied
/// creature aura carrying `MECHANIC_STUN` drive the two terms.
#[tokio::test]
async fn map_owned_player_melee_applies_victim_aurastate_and_mechanic_bonus_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    const MECHANIC_STUN: i8 = 12;
    const VICTIM_MECHANIC_SPELL: i32 = 70_002;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_931);
    let player_guid = ObjectGuid::create_player(1, 5_201);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Solo".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(creature_guid));
            unit.set_target(creature_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 5.0, 5.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    session.set_map_manager(Arc::clone(&manager));
    register_test_creature(&mut session, manager.clone(), creature_guid, 1_000);
    // C++ `Unit::IsTotem()` zeroes the victim's dodge, parry and block
    // (`Unit.cpp:2313-2360`); the attack table stays out of this damage-term
    // assertion.
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .add_unit_type_mask_like_cpp(wow_entities::UNIT_MASK_TOTEM);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, amount, misc) in [
        (
            91_110_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            100,
            i32::from(wow_entities::AURA_STATE_WOUNDED_20_PERCENT),
        ),
        (
            91_111_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            100,
            i32::from(MECHANIC_STUN),
        ),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura,
                    effect_misc_value_1: misc,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    spell_store.insert_spell_hit_metadata_for_difficulty_like_cpp(
        VICTIM_MECHANIC_SPELL,
        0,
        wow_data::SpellHitMetadataLikeCpp {
            spell_mechanic: MECHANIC_STUN,
            ..Default::default()
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));

    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let tick = |manager: &crate::map_manager::SharedMapManager,
                phase_state: &mut crate::session::PlayerMeleePhaseStateLikeCpp| {
        crate::session::run_legacy_player_melee_tick_once_like_cpp(
            manager,
            Some(&canonical),
            &attackers,
            2_000,
            phase_state,
            &config,
        )
    };
    let health = |manager: &crate::map_manager::SharedMapManager| {
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp()
    };

    // Baseline: no aura and a healthy victim, so both terms are neutral.
    let before = health(&manager);
    let outcome = tick(&manager, &mut phase_state);
    assert!(!outcome.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(before - health(&manager), 5, "weapon damage alone");

    // C++ `Unit::Update` sets `AURA_STATE_WOUNDED_20_PERCENT` below 20% health.
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.unit_mut().set_health(100);
        })
        .unwrap();
    session
        .apply_aura(91_110, player_guid, 30_000, 1)
        .expect("apply versus-aurastate aura");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    let before = health(&manager);
    let outcome = tick(&manager, &mut phase_state);
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(
        before - health(&manager),
        10,
        "SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE doubles the swing"
    );

    // The victim's applied STUN aura completes `HasAuraWithMechanic(1 << 12)`.
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    VICTIM_MECHANIC_SPELL as u32,
                    player_guid,
                    0,
                    0,
                ));
        })
        .unwrap();
    session
        .apply_aura(91_111, player_guid, 30_000, 1)
        .expect("apply versus-mechanic aura");
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    let before = health(&manager);
    let outcome = tick(&manager, &mut phase_state);
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(
        before - health(&manager),
        20,
        "the target-aura-mechanic term multiplies again"
    );
}

/// The map-owned melee phase applies `Unit::CalcArmorReducedDamage`, not only
/// the session path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1339`) runs the damage through
/// `CalcArmorReducedDamage` (`Unit.cpp:1623-1685`) before the hit table, using
/// the victim's `GetArmor()`. The production swing owner is the map-owned
/// runtime, so it must resolve the creature's `GenerateArmor` value and the
/// attacker's live armour-penetration inputs from the canonical Player.
#[tokio::test]
async fn map_owned_player_melee_applies_victim_armor_mitigation_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_932);
    let player_guid = ObjectGuid::create_player(1, 5_202);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "ArmorSolo".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(creature_guid));
            unit.set_target(creature_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 1_000.0, 1_000.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    session.set_map_manager(Arc::clone(&manager));
    register_test_creature(&mut session, manager.clone(), creature_guid, 100_000);
    session
        .mutate_world_creature(creature_guid, |creature| {
            // C++ `Unit::IsTotem()` zeroes the victim's dodge, parry and block
            // (`Unit.cpp:2313-2360`), isolating the damage term under test from
            // the attack table.
            creature
                .creature
                .add_unit_type_mask_like_cpp(wow_entities::UNIT_MASK_TOTEM);
            creature.creature.unit_mut().set_level(80);
            creature.creature.set_combat_log_stats_like_cpp(
                wow_entities::CreatureCombatLogStatsLikeCpp {
                    armor: 5_000,
                    ..Default::default()
                },
            );
        })
        .unwrap();

    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    // The attacker-side `SPELL_AURA_MOD_TARGET_RESISTANCE` sum is resolved from
    // the same spell store the melee bonus uses.
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::new(wow_data::SpellStore::new())),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let tick = |manager: &crate::map_manager::SharedMapManager,
                phase_state: &mut crate::session::PlayerMeleePhaseStateLikeCpp| {
        crate::session::run_legacy_player_melee_tick_once_like_cpp(
            manager,
            Some(&canonical),
            &attackers,
            2_000,
            phase_state,
            &config,
        )
    };
    let health = |manager: &crate::map_manager::SharedMapManager| {
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, creature_guid)
            .unwrap()
            .current_hp()
    };

    let before = health(&manager);
    let outcome = tick(&manager, &mut phase_state);
    assert!(!outcome.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(
        before - health(&manager),
        753,
        "C++ CalcArmorReducedDamage over a 1,000 damage roll and 5,000 armour"
    );

    // Same swing without armour stays exact, so the mitigation is the only
    // difference.
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_combat_log_stats_like_cpp(Default::default());
        })
        .unwrap();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    let before = health(&manager);
    let outcome = tick(&manager, &mut phase_state);
    assert_eq!(outcome.creature_hits, 1);
    assert_eq!(before - health(&manager), 1_000);
}

/// The map-owned melee phase applies `Unit::MeleeDamageBonusTaken`.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1334`) runs the victim's
/// `MeleeDamageBonusTaken` between the done bonus and the armour reduction. The
/// production swing owner is the map-owned runtime, so the creature's
/// `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` must be resolved there from the same
/// creature-aura projection the session uses.
#[tokio::test]
async fn map_owned_player_melee_applies_victim_melee_damage_taken_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let creature_guid = test_creature_guid(99_935);
    let player_guid = ObjectGuid::create_player(1, 5_205);
    let map_store = Arc::new(wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: 0,
        instance_type: wow_data::map::MAP_COMMON,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: 0,
        flags2: 0,
    }]));

    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::clone(&map_store));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TakenSolo".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_attacking(Some(creature_guid));
            unit.set_target(creature_guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 100.0, 100.0);
            unit.reset_attack_timer_like_cpp(WeaponAttackType::BaseAttack);
        })
        .unwrap();
    session.set_map_manager(Arc::clone(&manager));
    register_test_creature(&mut session, manager.clone(), creature_guid, 100_000);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_140, player_guid, 0, 1));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_140,
        wow_data::SpellInfo {
            spell_id: 91_140,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
                effect_base_points: -50,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);

    let (send_tx, _rx) = flume::bounded::<Vec<u8>>(8);
    let (command_tx, _crx) = flume::bounded::<SessionCommand>(8);
    let registration = PlayerRegistry::new().register_or_replace(
        player_guid,
        broadcast_info_with_command(player_guid, send_tx, command_tx),
        Default::default(),
    );
    let attackers = vec![crate::session::PlayerMeleeAttackerSnapshotLikeCpp {
        registration,
        player_guid,
        map_id: 0,
        instance_id: 0,
        in_combat_mirror: true,
        tap_group_guids: Vec::new(),
    }];
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    let mut phase_state = crate::session::PlayerMeleePhaseStateLikeCpp::default();
    let health_before = manager
        .read()
        .unwrap()
        .find_creature(0, 0, creature_guid)
        .unwrap()
        .current_hp();

    let outcome = crate::session::run_legacy_player_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &attackers,
        2_000,
        &mut phase_state,
        &config,
    );
    assert!(!outcome.skipped_owner_not_global, "the map owns this tick");
    assert_eq!(outcome.creature_hits, 1, "the swing resolves");
    assert_eq!(
        health_before
            - manager
                .read()
                .unwrap()
                .find_creature(0, 0, creature_guid)
                .unwrap()
                .current_hp(),
        50,
        "the -50 melee-damage-taken aura halves the 100 damage roll"
    );
}
