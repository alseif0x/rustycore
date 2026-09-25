//! Resistance, attack-power, and spell-power scenarios.

use super::*;

#[tokio::test]
async fn school_resistances_follow_update_resistances_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_500);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);

    // C++ `Unit::UpdateResistances` (`Unit.cpp:9148-9163`) with the aura
    // producers of `HandleAuraModResistance` and `HandleModResistancePercent`.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value, amount) in [
        (90_700, 22, 2, 20),   // MOD_RESISTANCE, holy mask
        (90_701, 22, 4, 30),   // MOD_RESISTANCE, fire mask
        (90_702, 101, 4, 100), // MOD_RESISTANCE_PCT, fire mask
        (90_703, 101, 2, 50),  // MOD_RESISTANCE_PCT, holy mask
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
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let resistances = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("resistance projection")
            .resistances
    };

    let _ = session.send_stat_update();
    assert_eq!(resistances(&session), [20, 0, 0, 0, 0, 0, 0]);

    for spell_id in [90_700, 90_701, 90_702, 90_703] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply resistance aura");
    }
    let _ = session.send_stat_update();
    // Holy: 20 flat * 1.5; fire: 30 flat * 2.0; the rest stay zero.
    assert_eq!(resistances(&session), [20, 30, 60, 0, 0, 0, 0]);

    let fire_slot = session
        .visible_aura_slot_for_spell_like_cpp(90_701)
        .expect("fire resistance aura slot");
    session
        .remove_aura(fire_slot)
        .expect("remove fire resistance aura");
    let _ = session.send_stat_update();
    assert_eq!(resistances(&session), [20, 30, 0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn attack_power_aura_producers_follow_update_attack_power_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_600);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    // C++ `HandleAuraModAttackPower`/`HandleAuraModAttackPowerPercent` and the
    // ranged variants (`SpellAuraEffects.cpp:4434-4492`).
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [
        (90_800, 99, 50),
        (90_801, 166, 50),
        (90_802, 124, 30),
        (90_803, 167, 100),
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
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("attack power projection")
    };

    let _ = session.send_stat_update();
    let baseline = stats(&session);
    assert_eq!(baseline.attack_power, 220);
    assert_eq!(baseline.attack_power_mod_pos, 0);
    assert_eq!(baseline.attack_power_multiplier, 0.0);
    assert_eq!(baseline.ranged_attack_power, -10);
    assert_eq!(baseline.ranged_attack_power_mod_pos, 0);
    assert_eq!(baseline.ranged_attack_power_multiplier, 0.0);

    for spell_id in [90_800, 90_801, 90_802, 90_803] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply attack power aura");
    }
    let _ = session.send_stat_update();
    let with_auras = stats(&session);
    assert_eq!(with_auras.attack_power_mod_pos, 50);
    assert_eq!(with_auras.attack_power_multiplier, 0.5);
    assert_eq!(with_auras.ranged_attack_power_mod_pos, 30);
    assert_eq!(with_auras.ranged_attack_power_multiplier, 1.0);
    assert_eq!(
        session.canonical_player_total_attack_power_like_cpp(),
        Some(405.0),
        "C++ GetTotalAttackPowerValue clamps the base plus modifier then applies the multiplier"
    );
}

#[tokio::test]
async fn override_attack_power_by_spell_power_aura_replaces_both_attack_mods_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_700);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    // C++ `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) feeds the
    // `ModHealingDonePos`/`ModDamageDonePos` fields the override reads.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 1_000,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, amount) in [(90_820, 15), (90_821, 5)] {
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
                aura_type: Some(
                    wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                ),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura:
                        wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("attack power projection")
    };

    let _ = session.send_stat_update();
    let baseline = stats(&session);
    assert_eq!(baseline.attack_power, 220);
    assert_eq!(baseline.ranged_attack_power, -10);

    // `ApplyModUpdateFieldValue` accumulates both active effects: 15 + 5.
    for spell_id in [90_820, 90_821] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply override attack power aura");
    }
    let _ = session.send_stat_update();
    let overridden = stats(&session);
    assert_eq!(
        overridden.attack_power, 200,
        "C++ replaces the strength/agility/level base with CalculatePct(1000 spell power, 20%)"
    );
    assert_eq!(overridden.ranged_attack_power, 200);
}

#[tokio::test]
async fn spell_damage_and_healing_bonus_auras_publish_update_spell_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_800);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    // C++ `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) accumulates
    // the item spell power into `GetBaseSpellPowerBonus()`.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 100,
            apply: true,
        }
    ));
    // C++ `ApplySpellPenetrationBonus` subtracts the item penetration from
    // `ModTargetResistance`.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPenetrationBonus {
            amount: 15,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value_1, misc_value_2, amount) in [
        // +30 holy damage, +20 fire damage and -50 fire damage.
        (90_900, 13, 1 << 1, 0, 30),
        (90_901, 13, 1 << 2, 0, 20),
        (90_902, 13, 1 << 2, 0, -50),
        // +40 flat healing for every school.
        (90_903, 135, 0, 0, 40),
        // +50% of intellect (40) as holy damage.
        (90_904, 174, 1 << 1, 3, 50),
        // +25% of spirit (30) as healing.
        (90_905, 175, 4, 0, 25),
        // +50% and +100% holy damage done, +25% fire damage done.
        (90_907, 79, 1 << 1, 0, 50),
        (90_908, 79, 1 << 1, 0, 100),
        (90_909, 79, 1 << 2, 0, 25),
        // +50% and +100% healing done.
        (90_910, 136, 0, 0, 50),
        (90_911, 136, 0, 0, 100),
        // Spell penetration aura (full magic mask) and armor-only penetration.
        (90_912, 123, 0x3E, 0, 20),
        (90_913, 123, 1, 0, 30),
        // Versatility bonus.
        (90_915, 471, 0, 0, 200),
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
                    effect_misc_value_1: misc_value_1,
                    effect_misc_value_2: misc_value_2,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    // `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (404) has no misc values.
    spell_store.insert(
        90_906,
        wow_data::SpellInfo {
            spell_id: 90_906,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    for spell_id in [
        90_900, 90_901, 90_902, 90_903, 90_904, 90_905, 90_907, 90_908, 90_909, 90_910, 90_911,
        90_912, 90_913, 90_915,
    ] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply spell bonus aura");
    }
    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("spell bonus projection");
    // Holy: 100 base + 30 flat + 50% of intellect 40.
    assert_eq!(stats.mod_damage_done_pos[1], 150);
    // Fire: (100 + 20 - 50) - (-50) leaves the positive aura only.
    assert_eq!(stats.mod_damage_done_pos[2], 120);
    assert_eq!(stats.mod_damage_done_neg[2], -50);
    assert_eq!(stats.mod_damage_done_pos[0], 0);
    // Healing: 100 base + 40 flat + intellect 40 (mana class) + 25% of spirit 30.
    assert_eq!(stats.mod_healing_done_pos, 187);
    // `HandleModDamagePercentDone` multiplies every matching effect:
    // holy (1 + 0.5) * (1 + 1.0) = 3.0, fire 1.25, the rest keep 1.0.
    assert_eq!(stats.mod_damage_done_percent[0], 1.0);
    assert_eq!(stats.mod_damage_done_percent[1], 3.0);
    assert_eq!(stats.mod_damage_done_percent[2], 1.25);
    assert_eq!(stats.mod_damage_done_percent[3..], [1.0; 4]);
    // `UpdateHealingDonePercentMod`: (1 + 0.5) * (1 + 1.0) = 3.0.
    assert_eq!(stats.mod_healing_done_percent, 3.0);
    // Aura 123 magic mask 20 minus item penetration 15; armor mask 30.
    assert_eq!(stats.mod_target_resistance, 5);
    assert_eq!(stats.mod_target_physical_resistance, 30);
    // No override aura is active yet: C++ fields hold the 0.0 default.
    assert_eq!(stats.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(stats.override_ap_by_spell_power_percent, 0.0);
    // `HandleModVersatilityByPct` sums aura 471 into `VersatilityBonus`.
    assert_eq!(stats.versatility_bonus, 200.0);

    // `HasAuraType` on 404 then replaces both attack mods with
    // `CalculatePct(min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX]), 50)`:
    // the minimum magic school is 100, so both become 50.
    session
        .apply_aura(90_906, player_guid, 30_000, 1)
        .expect("apply override attack power aura");
    let _ = session.send_stat_update();
    let overridden = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("override attack power projection");
    assert_eq!(overridden.attack_power, 50);
    assert_eq!(overridden.ranged_attack_power, 50);
    assert_eq!(
        overridden.mod_healing_done_pos, 187,
        "the attack-power override does not rewrite the spell fields"
    );
    assert_eq!(
        overridden.override_ap_by_spell_power_percent, 50.0,
        "the aura amount is published on ActivePlayerData"
    );
}

#[tokio::test]
async fn override_spell_power_by_ap_publishes_the_field_and_recomputes_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_850);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 100,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        90_920,
        wow_data::SpellInfo {
            spell_id: 90_920,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let _ = session.send_stat_update();
    let baseline = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("baseline projection");
    assert_eq!(baseline.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(baseline.mod_healing_done_pos, 100);
    assert_eq!(baseline.attack_power, 220);

    session
        .apply_aura(90_920, player_guid, 30_000, 1)
        .expect("apply override spell power aura");
    let _ = session.send_stat_update();
    let overridden = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("override spell power projection");
    assert_eq!(overridden.override_spell_power_by_ap_percent, 50.0);
    // `SpellBaseDamageBonusDone`/`SpellBaseHealingBonusDone` return
    // `int32(CalculatePct(melee AP 220, 50) + 0.5) = 110` and discard the item
    // spell power.
    assert_eq!(overridden.mod_healing_done_pos, 110);
    assert_eq!(overridden.mod_damage_done_pos[1..], [110; 6]);
    assert_eq!(overridden.attack_power, 220);
}
