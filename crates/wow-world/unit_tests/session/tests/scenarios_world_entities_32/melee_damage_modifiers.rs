//! Creature melee damage-modifier and expected-stat scenarios.

use super::*;

/// C++ `Unit::MeleeDamageBonusTaken` for a creature attacker against a player
/// victim, through the production ownership path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) runs the victim's
/// `MeleeDamageBonusTaken` before `CalcArmorReducedDamage`. For a white swing
/// the chain is the flat `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN` benefit, the
/// school-masked `SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN` and
/// `SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN_PCT` multipliers, and the Sanctified
/// Wrath bypass that the attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST`
/// shrinks (`Unit.cpp:7670-7778`).
#[test]
fn legacy_creature_melee_tick_once_applies_player_victim_taken_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_180);
    let creature_guid = test_creature_guid(91_181);

    let (mut session, _, _) = make_session();
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
        "TakenVictim".to_string(),
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
            player.unit_mut().set_max_health(100);
            player.unit_mut().set_health(100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.armor = 5_000;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_190_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_191,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MELEE_DAMAGE_TAKEN,
            5,
            0,
        ),
        (
            91_192,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            100,
            // Fire school: the normal-school chain ignores it.
            0x04,
        ),
        (
            91_193,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            100,
            // Normal school.
            0x01,
        ),
        (
            91_194,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN,
            -50,
            0x01,
        ),
        (
            91_195,
            wow_data::spell::aura_types::SPELL_AURA_MOD_IGNORE_TARGET_RESIST,
            50,
            0x01,
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
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = |canonical: &SharedCanonicalMapManager| {
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(player)
            .unwrap()
            .unit()
            .data()
            .health
    };
    let swing = |manager: &crate::map_manager::SharedMapManager, session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        let outcome =
            run_legacy_creature_melee_tick_once_like_cpp(manager, Some(&canonical), &config);
        outcome.commands.last().expect("one swing command").clone()
    };
    let aura_slot = |session: &WorldSession, spell_id: i32| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .iter()
                    .find(|(_, aura)| aura.spell_id == spell_id)
                    .map(|(slot, _)| *slot)
            })
            .flatten()
            .expect("applied aura slot")
    };

    // The `+5` victim aura zeroes the flat 5.0 miss band.
    session
        .apply_aura(91_190, player, 30_000, 1)
        .expect("apply hit-chance aura");

    // Baseline: 5,000 armour at level 80 -> 8.
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 92);

    // The flat `MOD_MELEE_DAMAGE_TAKEN` benefit: `(10 + 5)` -> armour -> 12.
    session
        .apply_aura(91_191, player, 30_000, 1)
        .expect("apply flat taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 12);
    assert_eq!(victim_health(&canonical), 80);

    // A non-normal `MOD_DAMAGE_PERCENT_TAKEN` row is filtered out.
    session
        .apply_aura(91_192, player, 30_000, 1)
        .expect("apply fire taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 12);
    assert_eq!(victim_health(&canonical), 68);

    // The normal-school row doubles `(10 + 5)` -> armour -> 23.
    session
        .apply_aura(91_193, player, 30_000, 1)
        .expect("apply normal taken aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 23);
    assert_eq!(victim_health(&canonical), 45);

    // Sanctified Wrath: with the victim's total modifier below one, the
    // attacker's `SPELL_AURA_MOD_IGNORE_TARGET_RESIST` shrinks the reduction
    // (`0.5 * (1 - 0.5) = 0.25`), and the same aura also halves the armour.
    session
        .remove_aura(aura_slot(&session, 91_193))
        .expect("remove normal taken aura");
    session
        .apply_aura(91_194, player, 30_000, 1)
        .expect("apply reduction aura");
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_195, player, 0, 1));
        })
        .unwrap();
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 10);
    assert_eq!(victim_health(&canonical), 35);
}

/// C++ `Player::GetBlockPercent`'s armour constant comes from the DB2
/// `ExpectedStat` table (`DB2Stores.cpp:2103-2173`), and the represented
/// runtime reads it through the aggro config's store handle.
#[test]
fn legacy_creature_melee_tick_once_reads_the_expected_stat_store_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let player = ObjectGuid::create_player(1, 91_300);
    let creature_guid = test_creature_guid(91_301);

    let (mut session, _, _) = make_session();
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
        "BlockDB2".to_string(),
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
            player.unit_mut().set_max_health(100_000);
            player.unit_mut().set_health(100_000);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.armor = 0;
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 100.0;
            stats.shield_block = 2_000;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10_000;
            creature.creature.ai_ownership_mut().max_damage = 10_000;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        91_310,
        wow_data::SpellInfo {
            spell_id: 91_310,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 5,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                effect_base_points: 5,
                ..Default::default()
            }],
        },
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .apply_aura(91_310, player, 30_000, 1)
        .expect("apply hit-chance aura");
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let expected_stat = |armor_constant: f32| {
        wow_data::ExpectedStatStore::from_entries([wow_data::ExpectedStatEntry {
            id: 0,
            expansion_id: -2,
            creature_health: 0.0,
            player_health: 0.0,
            creature_auto_attack_dps: 0.0,
            creature_armor: 0.0,
            player_mana: 0.0,
            player_primary_stat: 0.0,
            player_secondary_stat: 0.0,
            armor_constant,
            creature_spell_damage: 0.0,
            lvl: 80,
        }])
    };
    let tick = |session: &mut WorldSession,
                expected_stat_store: Option<Arc<wow_data::ExpectedStatStore>>| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
            spell_store: Some(Arc::clone(&spell_store)),
            expected_stat_store,
            ..Default::default()
        };
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config)
    };

    // No store: C++'s empty-store `EvaluateExpectedStat` fallback (`1.0`) gives
    // `min(2000 / 2001, 0.85) = 0.85`, so `10000 * 0.85 / 100 = 85` blocked.
    let outcome = tick(&mut session, None);
    assert_eq!(
        outcome.commands.last().expect("command").damage,
        10_000 - 85
    );

    // A level-80 row with `ArmorConstant = 8000` gives `2000 / 10000 = 0.2`,
    // so only `10000 * 0.2 / 100 = 20` is blocked.
    let outcome = tick(&mut session, Some(Arc::new(expected_stat(8_000.0))));
    assert_eq!(
        outcome.commands.last().expect("command").damage,
        10_000 - 20
    );
}

/// C++ `Unit::MeleeDamageBonusDone` (`Unit.cpp:7558-7650`) for a creature
/// attacker. The map-owned creature path must apply its own aura effects
/// against the victim's creature type before the victim-side mitigation and
/// outcome stages. This was previously the remaining #29 dead producer: the
/// creature aura list existed, but the melee owner discarded it for damage.
#[test]
fn legacy_creature_melee_tick_once_applies_creature_attacker_done_bonus_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(91_920);
    let victim_guid = test_creature_guid(91_921);

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    session
        .mutate_world_creature(victim_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_923,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_920_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_CREATURE,
            5_i32,
            1_i32 << 6,
        ),
        (
            91_921,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            100,
            1_i32 << 6,
        ),
        (
            91_923,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5,
            0,
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
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_920,
                    attacker_guid,
                    0,
                    1,
                ));
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(
                    91_921,
                    attacker_guid,
                    0,
                    1,
                ));
        })
        .unwrap();
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        creature_template_lifecycle_store: Some(Arc::new(
            wow_data::CreatureTemplateLifecycleStoreLikeCpp::from_templates([
                wow_data::CreatureTemplateLifecycleRecordLikeCpp {
                    entry: 9001,
                    creature_type: 7,
                    ..Default::default()
                },
            ]),
        )),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.canonical_creature_hits, 1);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim_guid, |victim| victim.unit().data().health)
            .unwrap(),
        70
    );
}
