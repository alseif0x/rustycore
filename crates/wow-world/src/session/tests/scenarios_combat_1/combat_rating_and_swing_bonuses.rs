use super::*;

#[test]
fn combat_rating_multiplier_uses_gt_table_like_cpp() {
    let (mut session, _, _) = make_session();
    assert_eq!(session.combat_rating_multiplier_like_cpp(80, 8), 1.0);

    let mut columns = [0.0f32; wow_data::CombatRatingsGameTableLikeCpp::VALUE_COLUMN_COUNT];
    columns[wow_data::CombatRatingsEntryLikeCpp::CRIT_MELEE] = 45.905987;
    columns[wow_data::CombatRatingsEntryLikeCpp::DODGE] = 45.250187;
    session.set_combat_ratings_game_table(Arc::new(
        wow_data::CombatRatingsGameTableLikeCpp::from_rows([
            wow_data::CombatRatingsEntryLikeCpp::from_columns(columns),
        ]),
    ));

    assert!((session.combat_rating_multiplier_like_cpp(1, 8) - (1.0 / 45.905987)).abs() < 0.00001);
    assert!((session.combat_rating_multiplier_like_cpp(1, 2) - (1.0 / 45.250187)).abs() < 0.00001);
    assert_eq!(
        session.combat_rating_multiplier_like_cpp(1, 23),
        1.0,
        "C++ ratings without a CombatRatings column use default multiplier"
    );
}

#[test]
fn white_swing_applies_autoattack_damage_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_030);
    let player = ObjectGuid::create_player(1, 84);

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
        "Swing".to_string(),
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
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let aura_spell_id = 90_997_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        aura_spell_id,
        wow_data::SpellInfo {
            spell_id: aura_spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 100,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_AUTOATTACK_DAMAGE,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    [crate::session::RepresentedMeleeDamageBonusLikeCpp::NONE; 2],
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    assert_eq!(
        swing(&mut session),
        Some(vec![7]),
        "the white swing uses the canonical effective weapon range"
    );

    session
        .apply_aura(aura_spell_id, player, 30_000, 1)
        .expect("apply autoattack damage aura");
    assert_eq!(
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().mod_autoattack_damage_pct_like_cpp()
            })
            .expect("canonical player"),
        2.0
    );
    assert_eq!(
        swing(&mut session),
        Some(vec![14]),
        "C++ MeleeDamageBonusDone adds the SPELL_AURA_MOD_AUTOATTACK_DAMAGE percentage"
    );

    session
        .remove_aura(0)
        .expect("remove autoattack damage aura");
    assert_eq!(swing(&mut session), Some(vec![7]));
}

#[test]
fn white_swing_applies_creature_type_melee_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_031);
    let player = ObjectGuid::create_player(1, 85);

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
        "Versus".to_string(),
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
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    // Template creature type 7 (undead) and the matching `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS`
    // and `SPELL_AURA_MOD_DAMAGE_DONE_CREATURE` effects.
    session.set_creature_template_lifecycle_store_like_cpp(Arc::new(
        wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                entry: 9001,
                creature_type: 7,
                ..Default::default()
            },
        ]),
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, amount) in [
        (
            90_998_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            100,
        ),
        (
            90_999_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            5,
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
                    effect_misc_value_1: 1 << 6,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        // Hoisted: the bonus resolves the canonical snapshot, so it must not run
        // inside the mutable owner borrow.
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    // Without the versus aura the flat creature-type benefit applies alone.
    let base = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(base[0].flat, 0);

    session
        .apply_aura(90_999, player, 30_000, 1)
        .expect("apply flat creature-type aura");
    let with_flat = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(with_flat[0].flat, 5);
    assert_eq!(with_flat[0].pct, 1.0);

    session
        .apply_aura(90_998, player, 30_000, 1)
        .expect("apply versus creature-type aura");
    let with_pct = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(with_pct[0].flat, 5);
    assert_eq!(with_pct[0].pct, 2.0);

    // `((7 + 5) * 2.0)`.
    assert_eq!(
        swing(&mut session),
        Some(vec![24]),
        "C++ MeleeDamageBonusDone"
    );
}

#[test]
fn white_swing_applies_victim_aurastate_and_mechanic_melee_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_032);
    let player = ObjectGuid::create_player(1, 86);
    // C++ `MECHANIC_STUN` (`SharedDefines.h:2552`).
    const MECHANIC_STUN: i32 = 12;
    // The victim aura carrier: a creature aura whose `SpellInfo::Mechanic` is
    // the STUN mechanic, so `Unit::HasAuraWithMechanic(1 << 12)` matches.
    const VICTIM_MECHANIC_SPELL: i32 = 70_001;

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
        "VictimState".to_string(),
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
            unit.set_attacking(Some(guid));
            unit.set_target(guid);
            unit.add_unit_state(UnitState::MELEE_ATTACKING.bits());
            unit.set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 2_000);
            unit.set_attack_timer(WeaponAttackType::BaseAttack, 0);
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 7.0, 7.0);
        })
        .unwrap();
    session.combat_target = Some(guid);
    session.in_combat = true;
    register_test_creature(&mut session, manager.clone(), guid, 40);
    session
        .mutate_world_creature(guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura, amount, misc) in [
        (
            91_100_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            100,
            i32::from(wow_entities::AURA_STATE_WOUNDED_20_PERCENT),
        ),
        (
            91_101_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            100,
            MECHANIC_STUN,
        ),
        (
            91_102_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            100,
            13,
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
            spell_mechanic: MECHANIC_STUN as i8,
            ..Default::default()
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let swing = |session: &mut WorldSession| {
        // Hoisted: the bonus resolves the canonical snapshot, so it must not run
        // inside the mutable owner borrow.
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_attack_timer(WeaponAttackType::BaseAttack, 0);
                take_canonical_player_attack_swings_like_cpp(
                    player,
                    0,
                    true,
                    true,
                    true,
                    melee_damage_bonus,
                    crate::session::combat::RepresentedArmorMitigationLikeCpp::NONE,
                    Default::default(),
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| {
                swings
                    .into_iter()
                    .map(|swing| swing.damage)
                    .collect::<Vec<_>>()
            })
    };

    // At full health the wounded-aurastate aura misses and no creature aura
    // carries the mechanic, so both terms stay neutral.
    session
        .apply_aura(91_100, player, 30_000, 1)
        .expect("apply versus-aurastate aura");
    session
        .apply_aura(91_101, player, 30_000, 1)
        .expect("apply versus-mechanic aura");
    session
        .apply_aura(91_102, player, 30_000, 1)
        .expect("apply non-matching versus-mechanic aura");
    let healthy = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(healthy[0].flat, 0);
    assert_eq!(healthy[0].pct, 1.0);

    // C++ `Unit::Update` sets `AURA_STATE_WOUNDED_20_PERCENT` for a living unit
    // below 20% health; 7/40 is below it.
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(7);
        })
        .expect("wounded victim");
    let wounded = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(wounded[0].flat, 0);
    assert_eq!(wounded[0].pct, 2.0);

    // The victim's STUN aura completes `HasAuraWithMechanic(1 << 12)`; the
    // FREEZE-misc aura does not.
    session
        .mutate_world_creature(guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    VICTIM_MECHANIC_SPELL as u32,
                    player,
                    0,
                    0,
                ));
        })
        .expect("victim mechanic aura");
    let stunned = session.represented_melee_damage_bonus_like_cpp();
    assert_eq!(stunned[0].flat, 0);
    assert_eq!(stunned[0].pct, 4.0);

    // `((7 + 0) * 4.0)`.
    assert_eq!(
        swing(&mut session),
        Some(vec![28]),
        "C++ MeleeDamageBonusDone"
    );
}
