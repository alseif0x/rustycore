use super::*;

#[test]
fn melee_attack_table_outcome_effects_match_calculate_melee_damage_like_cpp() {
    use crate::session_rules::{
        RepresentedMeleeOutcomeLikeCpp as Outcome, melee_outcome_damage_like_cpp,
        melee_outcome_presentation_like_cpp,
    };
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, HIT_INFO_CRITICAL_HIT, HIT_INFO_CRUSHING,
        HIT_INFO_GLANCING, HIT_INFO_MISS, HIT_INFO_OFFHAND, HIT_INFO_SWING_NO_HIT_SOUND,
        VICTIM_STATE_DODGE, VICTIM_STATE_EVADES, VICTIM_STATE_HIT, VICTIM_STATE_INTACT,
        VICTIM_STATE_PARRY,
    };

    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Evade, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Miss, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Dodge, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Parry, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Hit, 100, 80, 80, 1.0, 30.0),
        (100, 0, 100)
    );
    assert_eq!(
        // C++ assigns `OriginalDamage` after the doubling, so a critical swing
        // publishes the doubled value as its original too (`Unit.cpp:1370-1378`).
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80, 1.0, 30.0),
        (200, 0, 200)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crit, 100, 80, 80, 2.0, 30.0),
        (400, 0, 400)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Crushing, 101, 80, 80, 1.0, 30.0),
        (151, 0, 151)
    );
    // C++ `CalculatePct(damage, GetBlockPercent)` with the flat 30% creature
    // base (`Unit.h:947`).
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Block, 100, 80, 80, 1.0, 30.0),
        (70, 30, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Block, 7, 80, 80, 1.0, 30.0),
        (5, 2, 7)
    );
    // C++ `leveldif = min(victimLevel - attackerLevel, 3)` then
    // `reducePercent = 1 - leveldif * 0.1`.
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 81, 1.0, 30.0),
        (90, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 84, 1.0, 30.0),
        (70, 0, 100)
    );
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Glancing, 100, 80, 90, 1.0, 30.0),
        (70, 0, 100)
    );

    // C++ returns before any band on a physical-immune victim, publishing a
    // zero `HITINFO_NORMALSWING` with `VICTIMSTATE_IS_IMMUNE`.
    assert_eq!(
        melee_outcome_damage_like_cpp(Outcome::Immune, 100, 80, 80, 1.0, 30.0),
        (0, 0, 100)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Immune, false),
        (0, wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE)
    );

    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Evade, false),
        (
            HIT_INFO_MISS | HIT_INFO_SWING_NO_HIT_SOUND,
            VICTIM_STATE_EVADES
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Miss, false),
        (HIT_INFO_MISS, VICTIM_STATE_INTACT)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Dodge, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_DODGE)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Parry, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_PARRY)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Glancing, false),
        (
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_GLANCING,
            VICTIM_STATE_HIT
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Block, false),
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK, VICTIM_STATE_HIT)
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Crit, false),
        (
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRITICAL_HIT,
            VICTIM_STATE_HIT
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Crushing, false),
        (
            HIT_INFO_AFFECTS_VICTIM | HIT_INFO_CRUSHING,
            VICTIM_STATE_HIT
        )
    );
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Hit, false),
        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT)
    );
    // C++ sets `HITINFO_OFFHAND` before the table for `OFF_ATTACK`.
    assert_eq!(
        melee_outcome_presentation_like_cpp(Outcome::Hit, true),
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_OFFHAND, VICTIM_STATE_HIT)
    );
}

#[test]
fn white_swing_publishes_the_attack_table_outcome_like_cpp() {
    use crate::session_rules::melee_outcome_inputs_like_cpp;
    use wow_packet::packets::combat::{
        HIT_INFO_AFFECTS_VICTIM, HIT_INFO_MISS, VICTIM_STATE_DODGE, VICTIM_STATE_INTACT,
        VICTIM_STATE_PARRY,
    };

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_035);
    let player = ObjectGuid::create_player(1, 89);

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
        "Table".to_string(),
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
            // This test needs the real table, so it seeds the fixture's
            // creature avoidance and the untrained player's zero hit rating:
            // the C++ 5% miss, 3% dodge and 6% parry bands.
            creature
                .creature
                .set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                    dodge_pct: 3.0,
                    parry_pct: 6.0,
                    // Block has its own scenario; this one covers the
                    // damage-zeroing bands.
                    block_pct: 0.0,
                });
        })
        .unwrap();
    session
        .mutate_canonical_player_like_cpp(|player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.melee_hit_chance_pct = 0.0;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .unwrap();

    let swing = |session: &mut WorldSession| {
        let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
        let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
        let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
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
                    armor_mitigation,
                    outcome_facts,
                    crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
                )
            })
            .flatten()
            .map(|(swings, _)| swings)
    };

    // The creature is a normal level-2 victim facing the attacker, so the
    // represented table has 3% dodge and 6% parry bands and no miss band
    // (`5.0 - 7.5` clamps to zero). Over 400 landed swings both a full hit and
    // an avoided swing must appear.
    let mut hits = 0;
    let mut avoids = 0;
    for _ in 0..400 {
        let swings = swing(&mut session).expect("white swing resolves");
        assert_eq!(swings.len(), 1);
        if swings[0].damage == 0 {
            avoids += 1;
            assert!(
                swings[0].hit_info & HIT_INFO_MISS != 0
                    || swings[0].victim_state == VICTIM_STATE_DODGE
                    || swings[0].victim_state == VICTIM_STATE_PARRY,
                "an avoided swing must publish an avoid outcome: {:?}",
                swings[0]
            );
        } else {
            hits += 1;
            assert_eq!(swings[0].damage, 7);
            assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM);
        }
    }
    assert!(hits > 0, "some swings must land");
    assert!(avoids > 0, "the dodge/parry bands must be reachable");

    // A -200% `SPELL_AURA_MOD_HIT_CHANCE` makes the miss band exceed the whole
    // roll, so the swing is guaranteed to miss.
    let spell_id = 91_122_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: -200,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_HIT_CHANCE,
                effect_base_points: -200,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(spell_id, player, 30_000, 1)
        .expect("apply hit-chance aura");
    let facts = session.represented_melee_outcome_facts_like_cpp();
    assert_eq!(facts.0.hit_chance_aura_pct, -200.0);
    let inputs = melee_outcome_inputs_like_cpp(&facts.0, &facts.1);
    assert!(inputs[0].miss_chance_pct >= 100.0);

    let misses = swing(&mut session).expect("missed swing still resolves");
    assert_eq!(misses[0].damage, 0);
    // C++ keeps the post-armour `OriginalDamage` on an avoided swing.
    assert_eq!(misses[0].original_damage, 7);
    assert_eq!(misses[0].hit_info, HIT_INFO_MISS);
    assert_eq!(misses[0].victim_state, VICTIM_STATE_INTACT);
}

#[test]
fn white_swing_publishes_a_block_like_cpp() {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, HIT_INFO_BLOCK, VICTIM_STATE_HIT};

    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(18_036);
    let player = ObjectGuid::create_player(1, 90);

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
        "Block".to_string(),
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
            unit.set_weapon_damage(WeaponAttackType::BaseAttack, 100.0, 100.0);
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
            // Dodge and parry stay zero, so the 100% block band is the first
            // non-empty band after the (fixture-inert) miss band.
            creature
                .creature
                .set_avoidance_like_cpp(wow_entities::CreatureAvoidanceLikeCpp {
                    dodge_pct: 0.0,
                    parry_pct: 0.0,
                    block_pct: 100.0,
                });
        })
        .unwrap();

    let melee_damage_bonus = session.represented_melee_damage_bonus_like_cpp();
    let armor_mitigation = session.represented_melee_armor_mitigation_like_cpp();
    let outcome_facts = session.represented_melee_outcome_facts_like_cpp();
    let swings = session
        .mutate_canonical_player_like_cpp(|player| {
            take_canonical_player_attack_swings_like_cpp(
                player,
                0,
                true,
                true,
                true,
                melee_damage_bonus,
                armor_mitigation,
                outcome_facts,
                crate::session_rules::RepresentedMeleeDamageTakenLikeCpp::NONE,
            )
        })
        .flatten()
        .expect("white swing resolves")
        .0;
    assert_eq!(swings.len(), 1);
    // C++ `CalculatePct(100, GetBlockPercent)`: a flat 30% block.
    assert_eq!(swings[0].blocked, 30);
    assert_eq!(swings[0].damage, 70);
    assert_eq!(swings[0].hit_info, HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK);
    assert_eq!(swings[0].victim_state, VICTIM_STATE_HIT);
}
