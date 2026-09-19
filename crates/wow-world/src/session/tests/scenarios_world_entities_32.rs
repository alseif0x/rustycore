//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of `scenarios_world_entities_28.rs` under #29 when that file
//! reached 1,977 of the 2,000-line test-file budget; the parent module is
//! unchanged and the shared fixtures stay in `session_tests.rs`.

use super::*;

/// C++ `Unit::RollMeleeOutcomeAgainst`'s miss, dodge, parry and crit bands for
/// a creature attacker against a player victim, through the production
/// ownership path.
///
/// C++ `MeleeSpellMissChance` (`Unit.cpp:11652-11685`) starts from the victim's
/// flat `GetUnitMissChance()` of `5.0` and subtracts the creature attacker's
/// zero `m_modMeleeHitChance` (`Unit.cpp:360`), its
/// `SPELL_AURA_MOD_HIT_CHANCE` sum and the victim's
/// `SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE` sum; `GetUnitDodgeChance` and
/// `GetUnitParryChance` read the victim's published percentages and gate both
/// on `HasInArc(M_PI, attacker)`; the sitting-target rule returns a crit before
/// the avoidance bands (`Unit.cpp:2312-2314`). A missed or avoided swing
/// publishes zero dealt damage and no health transition.
#[test]
fn legacy_creature_melee_tick_once_resolves_player_victim_bands_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_CRITICAL_HIT, HIT_INFO_MISS, VICTIM_STATE_DODGE,
        VICTIM_STATE_HIT, VICTIM_STATE_INTACT, VICTIM_STATE_PARRY,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_140);
    let creature_guid = test_creature_guid(91_141);

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
        "Victim".to_string(),
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
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // The first stages assert an exact landed damage, so the creature's
            // flat 5% critical chance is disabled until the crit stage opts in.
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value_b) in [
        (
            91_150_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_151,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            -100,
            0,
        ),
        (
            91_152,
            wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH,
            100,
            // `!HealthBelowPct(50)` holds while the victim stays above half
            // health, which every stage keeps.
            50,
        ),
        (
            91_153,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
            0,
            // `MiscValue` is the school mask: the normal school.
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
                    effect_misc_value_1: if aura_type
                        == wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY
                    {
                        misc_value_b
                    } else {
                        0
                    },
                    effect_misc_value_2: misc_value_b,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
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
    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    // The fixture player's published parry is non-zero, so every stage sets the
    // avoidance it wants explicitly through the canonical victim's stats.
    let set_victim_avoidance = |session: &mut WorldSession, dodge: f32, parry: f32| {
        session
            .mutate_canonical_player_like_cpp(|player| {
                let mut stats = *player.effective_combat_stats_like_cpp();
                stats.dodge_pct = dodge;
                stats.parry_pct = parry;
                stats.block_pct = 0.0;
                player.replace_effective_combat_stats_like_cpp(stats);
            })
            .expect("canonical victim");
    };
    set_victim_avoidance(&mut session, 0.0, 0.0);

    // `+5` cancels the flat 5.0, so the swing is guaranteed to land.
    session
        .apply_aura(91_150, player, 30_000, 1)
        .expect("apply hit-chance aura");
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.swings_ready, 1);
    // The represented table resolves every outcome, so nothing is left to the
    // pre-table bridge.
    assert_eq!(outcome.melee_outcomes_unrepresented, 0);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 10);
    assert_eq!(outcome.commands[0].original_damage, 10);
    assert_eq!(outcome.commands[0].hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 90);

    // The `-100` sum makes the miss band cover the whole roll.
    session
        .apply_aura(91_151, player, 30_000, 1)
        .expect("apply miss aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.swings_ready, 1);
    assert_eq!(
        outcome.melee_outcomes_unrepresented, 0,
        "the represented miss band resolves the swing"
    );
    assert_eq!(outcome.canonical_hits, 0, "a miss commits no hit");
    assert_eq!(outcome.commands.len(), 1);
    assert_eq!(outcome.commands[0].damage, 0);
    // C++ keeps the post-armour `OriginalDamage` on an avoided swing.
    assert_eq!(outcome.commands[0].original_damage, 10);
    assert_eq!(outcome.commands[0].hit_info, HIT_INFO_MISS);
    assert_eq!(outcome.commands[0].victim_state, VICTIM_STATE_INTACT);
    assert_eq!(victim_health(&canonical), 90, "a miss deals no damage");

    // C++ `IsImmunedToDamage(SPELL_SCHOOL_MASK_NORMAL)` ends the swing before
    // every band (`Unit.cpp:1315-1324`): the packet keeps the zero
    // `HITINFO_NORMALSWING` and publishes `VICTIMSTATE_IS_IMMUNE` with no
    // damage.
    session
        .apply_aura(91_153, player, 30_000, 1)
        .expect("apply school-immunity aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands[0].clone();
    assert_eq!(outcome.canonical_hits, 0, "an immune swing commits no hit");
    assert_eq!(command.damage, 0);
    assert_eq!(command.hit_info, 0, "HITINFO_NORMALSWING is 0x0");
    assert_eq!(
        command.victim_state,
        wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE
    );
    assert_eq!(victim_health(&canonical), 90);
    // The later avoidance stages need the immunity gone again.
    let immunity_slot = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .find(|(_, aura)| aura.spell_id == 91_153)
                .map(|(slot, _)| *slot)
        })
        .flatten()
        .expect("immunity aura slot");
    session
        .remove_aura(immunity_slot)
        .expect("remove immunity aura");

    // The miss aura is removed again: the `+5` victim aura keeps the flat 5.0
    // band at zero, so only the avoidance/crit bands decide the later stages.
    let miss_slot = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .find(|(_, aura)| aura.spell_id == 91_151)
                .map(|(slot, _)| *slot)
        })
        .flatten()
        .expect("miss aura slot");
    session.remove_aura(miss_slot).expect("remove miss aura");
    let last_command = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        outcome.commands.last().expect("one swing command").clone()
    };

    // C++ `GetUnitDodgeChance` reads the victim's published `DodgePercentage`.
    set_victim_avoidance(&mut session, 100.0, 0.0);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(command.damage, 0);
    assert_eq!(command.original_damage, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(command.victim_state, VICTIM_STATE_DODGE);
    assert_eq!(victim_health(&canonical), 90);

    // C++ `GetUnitParryChance` uses the published `ParryPercentage`.
    set_victim_avoidance(&mut session, 0.0, 100.0);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 0);
    assert_eq!(command.damage, 0);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(command.victim_state, VICTIM_STATE_PARRY);
    assert_eq!(victim_health(&canonical), 90);

    // C++ `GetUnitBlockChance`'s player branch reads the published
    // `BlockPercentage`, and the blocked amount is
    // `CalculatePct(damage, Player::GetBlockPercent(attackerLevel))` — a
    // fraction over 100, so 8 damage over a 0.85 fraction truncates to 0 while
    // the swing still reports `HITINFO_BLOCK`.
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 100.0;
            stats.shield_block = 2_000;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.damage, 10);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | wow_packet::packets::combat::HIT_INFO_BLOCK
    );
    assert_eq!(victim_health(&canonical), 80);

    // C++ returns `MELEE_HIT_CRIT` before the avoidance bands for a player
    // victim that is not in a stand state while the critical chance is
    // non-zero, so the creature's flat 5% is enough once the flag is cleared.
    set_victim_avoidance(&mut session, 100.0, 100.0);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.creature.set_flags_extra_runtime_like_cpp(0);
        })
        .unwrap();
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_stand_state_like_cpp(wow_constants::UnitStandStateType::Sit);
        })
        .unwrap();
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.damage, 20);
    // C++ assigns `OriginalDamage` after the critical doubling.
    assert_eq!(command.original_damage, 20);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(command.victim_state, VICTIM_STATE_HIT);
    assert_eq!(victim_health(&canonical), 60);

    // Standing again, the victim's
    // `SPELL_AURA_MOD_CRIT_CHANCE_VERSUS_TARGET_HEALTH` aura over the whole
    // health range makes the critical band certain.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_stand_state_like_cpp(wow_constants::UnitStandStateType::Stand);
        })
        .unwrap();
    // With the avoidance bands empty the critical band decides.
    set_victim_avoidance(&mut session, 0.0, 0.0);
    session
        .apply_aura(91_152, player, 30_000, 1)
        .expect("apply crit-vs-health aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = last_command(&outcome);
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.damage, 20);
    assert_eq!(command.original_damage, 20);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT
    );
    assert_eq!(command.victim_state, VICTIM_STATE_HIT);
    assert_eq!(victim_health(&canonical), 40);
}

/// C++ `Unit::CalcArmorReducedDamage` for a creature attacker against a player
/// victim, through the production ownership path.
///
/// C++ `CalculateMeleeDamage` (`Unit.cpp:1326-1343`) mitigates through
/// `CalcArmorReducedDamage` before the outcome switch, reading the victim's
/// `GetArmor()` and the attacker's normal-school `MOD_TARGET_RESISTANCE`, while
/// the victim's `SPELL_AURA_BYPASS_ARMOR_FOR_CASTER` shrinks its own armour for
/// effects the attacker cast (`Unit.cpp:1631-1637`).
#[test]
fn legacy_creature_melee_tick_once_applies_player_victim_armor_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_160);
    let foreign_caster = ObjectGuid::create_player(1, 91_161);
    let creature_guid = test_creature_guid(91_162);

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
        "ArmoredVictim".to_string(),
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
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            // C++ `CalcArmorReducedDamage`'s `levelModifier` is the attacker's
            // level, so the fixture creature is a level-80 attacker.
            creature.creature.unit_mut().set_level(80);
            // The fixture asserts exact mitigated damage, so the flat 5% crit
            // stays off.
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
        91_170,
        wow_data::SpellInfo {
            spell_id: 91_170,
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
    spell_store.insert(
        91_171,
        wow_data::SpellInfo {
            spell_id: 91_171,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_BYPASS_ARMOR_FOR_CASTER,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
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
    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let swing = |manager: &crate::map_manager::SharedMapManager, session: &mut WorldSession| {
        reset_swing(session);
        let outcome =
            run_legacy_creature_melee_tick_once_like_cpp(manager, Some(&canonical), &config);
        outcome.commands.last().expect("one swing command").clone()
    };

    // The `+5` victim aura zeroes the flat 5.0 miss band.
    session
        .apply_aura(91_170, player, 30_000, 1)
        .expect("apply hit-chance aura");

    // 5,000 armour at level 80: `ceil(10 * (1 - 0.247127)) = 8`.
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(command.original_damage, 8);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(&canonical), 92);

    // A foreign caster's bypass aura is ignored (`GetCasterGUID() == attacker`).
    session
        .apply_aura(91_171, foreign_caster, 30_000, 1)
        .expect("apply foreign bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 8);
    assert_eq!(victim_health(&canonical), 84);

    // The attacker's own 50% bypass halves the armour: `ceil(10 * 0.859017) = 9`.
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 9);
    assert_eq!(victim_health(&canonical), 75);

    // Both bypass effects sum, removing the armour: the full 10 lands.
    let second_bypass = session
        .canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .iter()
                .filter(|(_, aura)| aura.spell_id == 91_171 && aura.caster_guid == foreign_caster)
                .map(|(slot, _)| *slot)
                .next()
        })
        .flatten()
        .expect("foreign bypass slot");
    session
        .remove_aura(second_bypass)
        .expect("remove foreign bypass aura");
    session
        .apply_aura(91_171, creature_guid, 30_000, 1)
        .expect("apply second attacker bypass aura");
    let command = swing(&manager, &mut session);
    assert_eq!(command.damage, 10);
    assert_eq!(command.original_damage, 10);
    assert_eq!(victim_health(&canonical), 65);
}

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

#[test]
fn player_block_percent_matches_get_block_percent_like_cpp() {
    use crate::session_rules::player_block_percent_like_cpp as percent;
    // C++ `Player::GetBlockPercent` (`Player.cpp:25288-25298`): a fraction
    // capped at `0.85`, and `0` when both inputs are zero.
    assert_eq!(percent(2_000, 1.0), 0.85);
    assert_eq!(percent(2_000, 8_000.0), 0.2);
    assert_eq!(percent(0, 0.0), 0.0);
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

/// C++ `Unit::CalculateMeleeDamage`'s school-absorb stage for a player victim
/// (`Unit.cpp:1449-1466`, `Unit::CalcAbsorbResist` `Unit.cpp:1789-1880`).
///
/// The map-owned swing spends the victim's `SPELL_AURA_SCHOOL_ABSORB` amount in
/// the same committed phase as the health write, publishes the absorbed amount
/// with the `HITINFO_FULL_ABSORB`/`HITINFO_PARTIAL_ABSORB` bit, and hands the
/// exhausted slot to the victim session, which owns the aura removal
/// (`Unit.cpp:1856-1860`) and its publication. A spent shield therefore stops
/// absorbing and the next swing lands at full damage.
#[test]
fn legacy_creature_melee_tick_once_absorbs_player_victim_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_FULL_ABSORB, HIT_INFO_PARTIAL_ABSORB,
    };

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_500);
    let creature_guid = test_creature_guid(91_501);

    let (mut session, _, send_rx) = make_session();
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let observer = ObjectGuid::create_player(1, 91_513);
    let (observer_send_tx, _observer_send_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(1);
    let mut observer_info =
        broadcast_info_with_command(observer, observer_send_tx, observer_command_tx);
    observer_info.placement.map_id = 0;
    observer_info.placement.instance_id = 0;
    observer_info.placement.position = Position::new(10.0, 10.0, 0.0, 0.0);
    registry.register_or_replace(observer, observer_info, Default::default());
    session.set_player_registry(Arc::clone(&registry));
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
        "Victim".to_string(),
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
        })
        .unwrap();
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_position(Position::new(10.0, 10.0, 0.0, 0.0));
            creature.creature.unit_mut().set_combat_reach(0.0);
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
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .expect("canonical victim");

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_510_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_511,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            // `MiscValue` is the school mask: the normal school.
            0x01,
        ),
        (
            91_512,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            4,
            0x01,
        ),
        (
            91_514,
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE,
            100_000,
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
                    effect_misc_value_2: 0,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_510, player, 30_000, 1)
        .expect("apply hit-chance aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let victim_health = || {
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
    let shield_amount = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == 91_511)
                    .map(|aura| {
                        (
                            aura.slot,
                            aura.represented_effect_amounts
                                .iter()
                                .find(|represented| represented.effect_index == 0)
                                .map(|represented| represented.amount),
                        )
                    })
            })
            .flatten()
    };

    session
        .apply_aura(91_511, player, 30_000, 1)
        .expect("apply absorb shield");
    let (shield_slot, _) = shield_amount(&session).expect("shield applied");

    // First swing: the 30-point shield absorbs the whole 10-point hit. C++
    // publishes the zero dealt damage with the full-absorb bit and no health
    // transition, and the shield keeps 20 points.
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(outcome.canonical_hits, 1);
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FULL_ABSORB
    );
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].slot, shield_slot);
    assert_eq!(command.absorb_consumptions[0].consumed, 10);
    assert!(
        !command.absorb_consumptions[0].removed,
        "a partially spent shield is not removed"
    );
    assert_eq!(victim_health(), 100, "a fully absorbed hit deals no damage");
    assert_eq!(shield_amount(&session).expect("shield").1, Some(20));

    // Second swing spends ten more and still publishes a full absorb.
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    assert_eq!(outcome.commands.last().expect("command").damage, 0);
    assert_eq!(shield_amount(&session).expect("shield").1, Some(10));

    // Third swing spends the last ten points and reports the exhausted shield.
    // Delivering the command publishes C++'s absorb log first
    // (`Unit.cpp:1876-1889`) and then removes the shield through the session's
    // aura transition (`Unit::CalcAbsorbResist`'s
    // `Remove(AURA_REMOVE_BY_ENEMY_SPELL)`).
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].slot, shield_slot);
    assert_eq!(command.absorb_consumptions[0].consumed, 10);
    assert!(command.absorb_consumptions[0].removed);
    assert_eq!(victim_health(), 100);
    let _ = drain_server_opcodes(&send_rx);
    session.state = crate::session::SessionState::LoggedIn;
    session.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
    assert_eq!(
        shield_amount(&session),
        None,
        "the delivered command removes the spent shield"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    let absorb_log = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::SpellAbsorbLog)
        .expect("C++ publishes one absorb log per consuming shield");
    let removal = opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AuraUpdate)
        .expect("the session-owned transition publishes the shield removal");
    assert!(
        absorb_log < removal,
        "C++ logs the absorb before removing the spent shield (`Unit.cpp:1876-1889`)"
    );
    let SessionCommand::SendRealmIfVisibleLikeCpp(observer_command) = observer_command_rx
        .try_recv()
        .expect("C++ fans the absorb log to nearby visible players")
    else {
        panic!("the absorb log must use the realm-visible observer rail");
    };
    assert_eq!(observer_command.source_guid, player);
    assert_eq!(observer_command.map_id, 0);
    assert_eq!(observer_command.instance_id, 0);
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&observer_command.packet_bytes).server_opcode(),
        Some(ServerOpcodes::SpellAbsorbLog),
        "the observer receives the same C++ SpellAbsorbLog frame"
    );

    // Fourth swing: with the shield gone the full hit lands.
    session
        .apply_aura(91_514, player, 30_000, 1)
        .expect("apply a large elemental resistance aura");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 0);
    assert_eq!(command.damage, 10);
    // C++ `CalcSpellResistedDamage` returns zero before reading resistances for
    // a non-magic school (`Unit.cpp:1688-1693`); a physical white swing must
    // remain a ten-point hit despite the enormous aura above.
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(), 90);

    // A shield smaller than the hit leaves a partial absorb: the 4-point shield
    // spends everything and the remaining 6 points land.
    session
        .apply_aura(91_512, player, 30_000, 1)
        .expect("apply small absorb shield");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 4);
    assert_eq!(command.damage, 6);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(command.absorb_consumptions.len(), 1);
    assert_eq!(command.absorb_consumptions[0].consumed, 4);
    assert!(command.absorb_consumptions[0].removed);
    assert_eq!(victim_health(), 84);
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

/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop for a player victim
/// (`Unit.cpp:1886-1930`).
///
/// The map-owned swing drains the victim's mana in the same committed phase as
/// the health write, publishes the resulting `SMSG_POWER_UPDATE` through the
/// victim session on delivery, and leaves the shield's amount spent so the mana
/// it can drain is finite.
#[test]
fn legacy_creature_melee_tick_once_drains_player_mana_shield_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_600);
    let creature_guid = test_creature_guid(91_601);

    let (mut session, _, send_rx) = make_session();
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
        "Victim".to_string(),
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
            player.set_power_index(wow_constants::PowerType::Mana, Some(0));
            player
                .unit_mut()
                .set_max_power(wow_constants::PowerType::Mana, 100);
            player
                .unit_mut()
                .set_power(wow_constants::PowerType::Mana, 100);
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
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
    for (spell_id, aura_type, amount, amplitude, misc_value) in [
        (
            91_610_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0.0_f32,
            0_i32,
        ),
        (
            91_611,
            wow_data::spell::aura_types::SPELL_AURA_MANA_SHIELD,
            30,
            // One point of mana per point of damage absorbed.
            1.0,
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
                    effect_base_points: amount,
                    effect_amplitude: amplitude,
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: 0,
                    ..Default::default()
                }],
            },
        );
    }
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_610, player, 30_000, 1)
        .expect("apply hit-chance aura");
    session
        .apply_aura(91_611, player, 30_000, 1)
        .expect("apply mana shield aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let victim_health = || {
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
    let victim_mana = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player.unit().get_power(wow_constants::PowerType::Mana)
            })
            .expect("canonical victim")
    };
    let shield_amount = |session: &WorldSession| {
        session
            .canonical_player_snapshot_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == 91_611)
                    .map(|aura| {
                        (
                            aura.slot,
                            aura.represented_effect_amounts
                                .iter()
                                .find(|represented| represented.effect_index == 0)
                                .map(|represented| represented.amount),
                        )
                    })
            })
            .flatten()
    };

    // Plenty of mana: the whole hit is absorbed and the drain is published.
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(command.mana_spent, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM | 0x20);
    assert_eq!(victim_health(), 100);
    assert_eq!(victim_mana(&session), 90);
    assert_eq!(shield_amount(&session).expect("shield").1, Some(20));
    let _ = drain_server_opcodes(&send_rx);
    session.state = crate::session::SessionState::LoggedIn;
    session.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::PowerUpdate),
        "C++ ModifyPower publishes the drained mana"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::SpellAbsorbLog),
        "the mana shield publishes its absorb log"
    );

    // Only three mana left: the shield absorbs what the victim can pay and the
    // remainder lands.
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(player)
        .unwrap()
        .unit_mut()
        .set_power(wow_constants::PowerType::Mana, 3);
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.mana_spent, 3);
    assert_eq!(command.absorbed, 3);
    assert_eq!(command.damage, 7);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(victim_health(), 93);
    assert_eq!(victim_mana(&session), 0);
    // Stage 1 left 20 points, and this one spent the three the victim could pay.
    assert_eq!(shield_amount(&session).expect("shield").1, Some(17));

    // No mana: the shield absorbs nothing and the full hit lands.
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.mana_spent, 0);
    assert_eq!(command.absorbed, 0);
    assert_eq!(command.damage, 10);
    assert_eq!(command.hit_info, HIT_INFO_AFFECTS_VICTIM);
    assert_eq!(victim_health(), 83);
}

/// C++ `Unit::CalcAbsorbResist`'s ignore-absorb term (`Unit.cpp:1803-1832`).
///
/// A creature attacker carrying `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` may only
/// push the non-ignored portion of its hit into the victim's school-absorb
/// shield, unless that shield's spell carries
/// `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
#[test]
fn legacy_creature_melee_tick_once_honors_ignore_absorb_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_PARTIAL_ABSORB};

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);

    let player = ObjectGuid::create_player(1, 91_800);
    let creature_guid = test_creature_guid(91_801);

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
        "Victim".to_string(),
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
            stats.dodge_pct = 0.0;
            stats.parry_pct = 0.0;
            stats.block_pct = 0.0;
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
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            // C++ `GetMaxPositiveAuraModifierByMiscMask` reads the attacker's
            // own `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL` effects.
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .add_applied(wow_entities::AppliedAuraRef::new(91_812_u32, player, 0, 1));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            91_810_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            5_i32,
            0_i32,
        ),
        (
            91_811,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
            0x01,
        ),
        (
            91_812,
            wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL,
            50,
            0x01,
        ),
        (
            91_813,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            30,
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
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: 0,
                    ..Default::default()
                }],
            },
        );
    }
    // The protected shield carries `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE`.
    let mut attributes = [0_u32; 15];
    attributes[6] = wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE;
    spell_store.insert_spell_misc_attributes_like_cpp(91_813, attributes);
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));
    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(Arc::clone(&spell_store)),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    session
        .apply_aura(91_810, player, 30_000, 1)
        .expect("apply hit-chance aura");

    let reset_swing = |session: &mut WorldSession| {
        session
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
    };
    let victim_health = || {
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

    // A 30-point shield without the attribute may only take the half the
    // attacker's 50% modifier leaves: 5 of the 10-point hit.
    session
        .apply_aura(91_811, player, 30_000, 1)
        .expect("apply absorb shield");
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 5);
    assert_eq!(command.damage, 5);
    assert_eq!(
        command.hit_info,
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB
    );
    assert_eq!(victim_health(), 95);

    // The shield whose spell carries
    // `SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE` ignores the modifier and absorbs the
    // whole hit.
    session
        .apply_aura(91_813, player, 30_000, 1)
        .expect("apply protected absorb shield");
    reset_swing(&mut session);
    let outcome = run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config);
    let command = outcome.commands.last().expect("command").clone();
    assert_eq!(command.absorbed, 10);
    assert_eq!(command.damage, 0);
    assert_eq!(victim_health(), 95);
}
