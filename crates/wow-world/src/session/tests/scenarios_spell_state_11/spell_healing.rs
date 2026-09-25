//! Direct and missing-health spell-healing scenarios.

use super::*;

#[tokio::test]
async fn spell_direct_heal_applies_spell_power_and_healing_percent_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 727_i32;
    let guid = ObjectGuid::create_player(1, 58);
    let canonical = shared_canonical_map_manager();
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
        guid,
        "HealScaler".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 1_000);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    // `SpellBaseHealingBonusDone`: 100 base spell power, no mana slot, and
    // `ModHealingDonePercent = 1.5`.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    spell_power: 100,
                    base_mana: 0,
                    stats: [10, 10, 10, 40, 30],
                    mod_healing_done_percent: 1.5,
                    ..Default::default()
                },
            );
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(100);
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            // Holy.
            school_mask: 1 << 1,
            ..Default::default()
        },
    ])));

    // The heal plus the victim `SPELL_AURA_MOD_HEALING` (40 holy) and the
    // `SPELL_AURA_MOD_HEALING_PCT` -50/+25 auras applied below.
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    for (aura_spell_id, aura_type, misc_value, amount) in [
        (
            90_950_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING,
            1 << 1,
            40,
        ),
        (
            90_951_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_PCT,
            0,
            -50,
        ),
        (
            90_952_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_PCT,
            0,
            25,
        ),
        (
            90_953_i32,
            wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH,
            0,
            100,
        ),
    ] {
        spell_store.insert(
            aura_spell_id,
            wow_data::SpellInfo {
                spell_id: aura_spell_id,
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
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented direct heal should scale");

    // `int32((100 + int32(100 * 0.5)) * 1.5)`.
    assert_eq!(session.player_health_like_cpp(), 325);

    // C++ `SpellHealingBonusDone` also adds the victim's
    // `SPELL_AURA_MOD_HEALING` by school mask; the self-heal victim is the
    // session player, whose auras are represented.
    session
        .apply_aura(90_950, guid, 30_000, 1)
        .expect("apply victim healing aura");
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("second represented direct heal should include the victim aura");

    // The second cast: `int32((100 + int32(140 * 0.5)) * 1.5) = 255`.
    assert_eq!(session.player_health_like_cpp(), 580);

    // C++ `SpellHealingBonusTaken`: the most negative (-50) and most positive
    // (+25) `SPELL_AURA_MOD_HEALING_PCT` amounts multiply the received heal.
    for aura_spell_id in [90_951_i32, 90_952_i32] {
        session
            .apply_aura(aura_spell_id, guid, 30_000, 1)
            .expect("apply healing taken aura");
    }
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("third represented direct heal should apply the taken modifiers");

    // The third cast: `int32(255 * (1 - 0.5) * (1 + 0.25)) = 159`.
    assert_eq!(session.player_health_like_cpp(), 739);

    // C++ `SpellHealingPctDone`: the target's missing health scales healing for
    // `SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH` (100 -> +50% at
    // half health). The taken auras are removed so the assertions stay exact.
    for aura_spell_id in [90_951_i32, 90_952_i32] {
        let slot = session
            .visible_aura_slot_for_spell_like_cpp(aura_spell_id)
            .expect("taken aura slot");
        session.remove_aura(slot).expect("remove taken aura");
    }
    session
        .apply_aura(90_953, guid, 30_000, 1)
        .expect("apply missing-health healing aura");
    session.set_player_health_like_cpp(500, 1_000);
    session
        .execute_spell(spell_id, guid)
        .await
        .expect("fourth represented direct heal should scale by missing health");

    // `int32((100 + int32(140 * 0.5)) * (1.5 * (1 + (100 * 50 / 100) / 100)))`
    // `= int32(170 * 2.25) = 382`.
    assert_eq!(session.player_health_like_cpp(), 882);
}

#[tokio::test]
async fn spell_direct_heal_scales_by_creature_missing_health_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 729_i32;
    let guid = test_creature_guid(18_016);
    let player_guid = ObjectGuid::create_player(1, 60);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    session
        .mutate_world_creature(guid, |creature| {
            creature.creature.unit_mut().set_health(500);
        })
        .expect("damaged creature");

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            school_mask: 1 << 1,
            ..Default::default()
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    let aura_spell_id = 90_970_i32;
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
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session
        .apply_aura(aura_spell_id, player_guid, 30_000, 1)
        .expect("apply missing-health aura");

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented creature heal should scale by missing health");

    // The creature is at 50% health, so the aura adds `100 * 50 / 100 = 50`:
    // `int32(100 * 1.5) = 150`.
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 0, guid)
            .unwrap()
            .current_hp(),
        650
    );
}

#[tokio::test]
async fn spell_self_heal_syncs_player_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 44);
    let canonical = shared_canonical_map_manager();
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
        guid,
        "Healer".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(50, 100);
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

    session.apply_heal(None, guid, 20).await.unwrap();

    assert_eq!(session.player_health_like_cpp(), 70);
    let canonical_health = session
        .mutate_canonical_player_like_cpp(|player| player.unit().data().health)
        .unwrap();
    assert_eq!(canonical_health, 70);
    let sent = send_rx.try_recv().unwrap();
    let opcode = u16::from_le_bytes([sent[0], sent[1]]);
    assert_eq!(opcode, ServerOpcodes::UpdateObject as u16);
}
#[tokio::test]
async fn spell_self_heal_skips_dead_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 46);
    session.set_player_guid(Some(guid));
    session.set_player_health_like_cpp(0, 100);

    session.apply_heal(None, guid, 20).await.unwrap();

    assert_eq!(session.player_health_like_cpp(), 0);
    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.try_recv().is_err());
}
