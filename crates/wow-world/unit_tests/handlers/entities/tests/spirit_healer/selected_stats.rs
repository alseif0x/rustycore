use super::*;
use std::collections::BTreeSet;
use std::sync::Arc;
use wow_constants::DeathState;
use wow_data::spell::aura_types::{
    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE, SPELL_AURA_MOD_PERCENT_STAT,
    SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE, SPELL_AURA_NO_PVP_CREDIT,
};
use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

fn sickness_effect_rows(
    variants: &[(u8, i32, i32, i32, i32)],
) -> Vec<wow_data::spell_db2::SpellEffectDb2Entry> {
    variants
        .iter()
        .flat_map(
            |&(difficulty, stat_points, damage_points, stat_misc, damage_misc)| {
                [
                    (0, SPELL_AURA_MOD_PERCENT_STAT, stat_points, stat_misc),
                    (
                        1,
                        SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
                        damage_points,
                        damage_misc,
                    ),
                    (2, SPELL_AURA_NO_PVP_CREDIT, -1, 127),
                ]
                .into_iter()
                .map(move |(effect_index, aura_type, base_points, misc_value)| {
                    let mut row = super::selected_execution_effects::effect_row(
                        30_000 + u32::from(difficulty) * 10 + effect_index,
                        difficulty,
                        effect_index,
                        SPELL_EFFECT_APPLY_AURA,
                        aura_type,
                        base_points,
                    );
                    row.effect_misc_value = [misc_value, 0];
                    row
                })
            },
        )
        .collect()
}

fn sickness_misc_row(difficulty: u8) -> wow_data::SpellMiscEntry {
    let mut attributes = [0; 15];
    attributes[0] = 0x2C80_0100;
    attributes[1] = 0x88;
    wow_data::SpellMiscEntry {
        id: 31_000 + u32::from(difficulty),
        spell_id: SICKNESS_SPELL as u32,
        difficulty_id: difficulty,
        duration_index: 427,
        attributes: attributes.map(|value| value as i32),
        ..Default::default()
    }
}

fn install_stats_catalog(session: &mut WorldSession) {
    let (race, class_id, level) = {
        let player = crate::session::hub_ref(session);
        (
            player.player_race_like_cpp(),
            player.player_class_like_cpp(),
            player.player_level_like_cpp(),
        )
    };
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (race, class_id, level),
        wow_data::PlayerLevelStats {
            strength: 100,
            agility: 100,
            stamina: 100,
            intellect: 100,
            spirit: 100,
            base_mana: 500,
        },
    )])));
    let mut class = wow_data::character_progression::ChrClassesEntry::default();
    class.id = u32::from(class_id);
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([class]),
    ));
}

fn clear_auras(session: &mut WorldSession) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.subsystems_mut().auras = wow_entities::AuraSubsystem::default();
            unit.set_death_state(DeathState::Alive);
        })
        .expect("canonical Player");
}

fn application(
    spell_id: i32,
    difficulty_id: u8,
    slot: u8,
    effect_mask: u32,
    amounts: &[(u8, i32)],
) -> wow_entities::AuraApplicationLikeCpp {
    wow_entities::AuraApplicationLikeCpp {
        spell_id,
        difficulty_id,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 60_000,
        duration_remaining: 60_000,
        stack_count: 1,
        aura_flags: 0,
        effect_mask,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: amounts
            .iter()
            .map(
                |&(effect_index, amount)| wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index,
                    amount,
                },
            )
            .collect(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn legacy_spell(
    spell_id: i32,
    aura_type: i32,
    misc_value: i32,
    misc_value_b: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: SPELL_EFFECT_APPLY_AURA,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_misc_value_2: misc_value_b,
            ..Default::default()
        }],
    }
}

fn insert_application(session: &mut WorldSession, aura: wow_entities::AuraApplicationLikeCpp) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .insert_runtime_application_like_cpp(aura);
        })
        .expect("canonical Player");
}

fn effective_stats(session: &WorldSession) -> wow_entities::PlayerEffectiveCombatStatsLikeCpp {
    session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("canonical effective stats")
}

async fn three_only_sickness_session(
    guid_counter: i64,
    map_difficulty: u8,
    effect_rows: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
    difficulties: Vec<wow_data::DifficultyEntry>,
    misc_rows: Vec<wow_data::SpellMiscEntry>,
    visual_difficulty: u8,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    super::selected_execution_effects::selected_session_with_visual_inputs(
        guid_counter,
        map_difficulty,
        effect_rows,
        difficulties,
        None,
        Some(misc_rows),
        visual_difficulty,
    )
    .await
}

#[tokio::test]
async fn three_only_sickness_updates_native_stats_damage_and_restores_on_expiry() {
    let (mut session, _send_rx, player_guid) = three_only_sickness_session(
        43_971,
        3,
        sickness_effect_rows(&[(3, -20, -40, -1, 127)]),
        vec![super::selected_execution_effects::difficulty(3, 0)],
        vec![sickness_misc_row(3)],
        3,
    )
    .await;
    let store = session.catalogs.spell_store().expect("SpellStore");
    assert!(store.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 3));
    assert!(!store.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 0));
    assert!(store.get(SICKNESS_SPELL).is_none());

    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_stats_catalog(&mut session);
    clear_auras(&mut session);
    assert!(session.send_stat_update());
    let baseline = effective_stats(&session);
    assert_eq!(baseline.stats, [100; 5]);
    assert_eq!(baseline.weapon_damage_pct, [1.0, 0.5, 1.0]);
    assert_eq!(baseline.mod_damage_done_percent, [1.0; 7]);

    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("fresh sickness uses its difficulty-three canonical rows");
    let penalized = effective_stats(&session);
    assert_eq!(penalized.stats, [80; 5]);
    assert_eq!(penalized.weapon_damage_pct, [0.6, 0.0, 0.6]);
    assert_eq!(penalized.weapon_damage[1], [0.0, 0.0]);
    assert_eq!(penalized.mod_damage_done_percent, [0.6; 7]);

    let slot = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL)
                .expect("fresh canonical sickness")
                .slot
        })
        .expect("canonical Player");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(slot)
                .expect("fresh sickness application")
                .duration_remaining = 1;
        })
        .expect("canonical Player");
    let expired = wow_world_spell::tick_selected_aura_lifetimes(&mut session, 1)
        .expect("sickness expiry uses canonical selected removal");
    assert_eq!(expired.expired, 1);
    let restored = effective_stats(&session);
    assert_eq!(restored.stats, baseline.stats);
    assert_eq!(restored.weapon_damage_pct, [1.0, 0.0, 1.0]);
    assert_eq!(restored.weapon_damage[1], [0.0, 0.0]);
    assert_eq!(
        restored.mod_damage_done_percent,
        baseline.mod_damage_done_percent
    );
}

#[tokio::test]
async fn refresh_uses_original_aura_rows_and_native_amounts_after_map_difficulty_changes() {
    let (mut session, _send_rx, player_guid) = three_only_sickness_session(
        43_972,
        0,
        sickness_effect_rows(&[(0, -10, -20, -1, 127), (3, -50, -60, 0, 4)]),
        vec![
            super::selected_execution_effects::difficulty(0, 3),
            super::selected_execution_effects::difficulty(3, 0),
        ],
        vec![sickness_misc_row(0), sickness_misc_row(3)],
        0,
    )
    .await;
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_stats_catalog(&mut session);
    clear_auras(&mut session);
    assert!(session.send_stat_update());

    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("create the original difficulty-zero sickness");
    let original = effective_stats(&session);
    assert_eq!(original.stats, [90; 5]);
    assert_eq!(original.mod_damage_done_percent, [0.8; 7]);

    super::selected_execution_effects::install_player_on_difficulty_map(&mut session, 3);
    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("refresh from the current difficulty while retaining original Aura data");
    let refreshed = effective_stats(&session);
    let (slot, raw_difficulty, snapshot) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let aura = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL)
                .expect("refreshed sickness");
            (
                aura.slot,
                aura.difficulty_id,
                auras
                    .threat_snapshot_like_cpp(aura.slot)
                    .expect("refreshed effect snapshot")
                    .effects()
                    .to_vec(),
            )
        })
        .expect("canonical Player");
    assert_eq!(raw_difficulty, 0);
    assert_eq!(snapshot.iter().find(|effect| effect.0 == 1).unwrap().2, -50);
    assert_eq!(snapshot.iter().find(|effect| effect.0 == 2).unwrap().2, -60);
    assert_eq!(snapshot.iter().find(|effect| effect.0 == 1).unwrap().3, -1);
    assert_eq!(snapshot.iter().find(|effect| effect.0 == 2).unwrap().3, 127);

    let refreshed_damage_multiplier = 1.0_f32 - 60.0_f32 / 100.0_f32;
    assert_eq!(refreshed.stats, [50; 5]);
    assert_eq!(
        refreshed.weapon_damage_pct,
        [
            refreshed_damage_multiplier,
            0.0,
            refreshed_damage_multiplier
        ]
    );
    assert_eq!(refreshed.weapon_damage[1], [0.0, 0.0]);
    assert_eq!(
        refreshed.mod_damage_done_percent,
        [refreshed_damage_multiplier; 7]
    );

    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let aura = player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(slot)
                .expect("refreshed sickness application");
            aura.represented_effect_amounts = vec![
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 0,
                    amount: -35,
                },
                wow_entities::RepresentedAuraEffectAmountLikeCpp {
                    effect_index: 1,
                    amount: -45,
                },
            ];
        })
        .expect("canonical Player");
    assert!(session.send_stat_update());
    let represented = effective_stats(&session);
    assert_eq!(represented.stats, [65; 5]);
    assert_eq!(represented.weapon_damage_pct, [0.55, 0.0, 0.55]);
    assert_eq!(represented.weapon_damage[1], [0.0, 0.0]);
    assert_eq!(represented.mod_damage_done_percent, [0.55; 7]);
}

#[tokio::test]
async fn keyless_configured_sickness_payload_is_rejected_before_stat_mutation() {
    let effect_rows = sickness_effect_rows(&[(0, -21, -41, -1, 127)]);
    let (mut session, _send_rx, _) =
        super::selected_execution_effects::selected_session_without_selected_spell_info_key(
            43_973,
            0,
            effect_rows,
            vec![super::selected_execution_effects::difficulty(0, 0)],
            None,
            None,
            0,
        )
        .await;
    let store = session.catalogs.spell_store().expect("SpellStore");
    assert!(store.get(SICKNESS_SPELL).is_some());
    assert!(!store.contains_spell_info_any_difficulty_like_cpp(SICKNESS_SPELL as u32));

    let mut other_race = super::corpse::human_race_entry_with_faction(1);
    other_race.id = 99;
    other_race.res_sickness_spell_id = SICKNESS_SPELL;
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([other_race]),
    ));
    install_stats_catalog(&mut session);
    clear_auras(&mut session);
    assert!(session.send_stat_update());
    let baseline = effective_stats(&session);

    insert_application(
        &mut session,
        application(SICKNESS_SPELL, 0, 5, 0b111, &[(0, -20), (1, -40)]),
    );
    assert!(
        crate::session::hub_ref(&session)
            .resolved_represented_total_stat_multipliers_like_cpp()
            .is_none()
    );
    assert!(!session.send_stat_update());
    assert_eq!(effective_stats(&session), baseline);
}

#[tokio::test]
async fn broken_known_sickness_difficulty_graph_is_rejected_before_stat_mutation() {
    let (mut session, _send_rx, _) = three_only_sickness_session(
        43_974,
        7,
        sickness_effect_rows(&[(3, -21, -41, -1, 127)]),
        vec![
            super::selected_execution_effects::difficulty(3, 0),
            super::selected_execution_effects::difficulty(7, 8),
            super::selected_execution_effects::difficulty(8, 7),
        ],
        vec![sickness_misc_row(3)],
        3,
    )
    .await;
    set_race_sickness_spell(&mut session, SICKNESS_SPELL);
    install_stats_catalog(&mut session);
    clear_auras(&mut session);
    assert!(session.send_stat_update());
    let baseline = effective_stats(&session);

    insert_application(
        &mut session,
        application(SICKNESS_SPELL, 7, 5, 0b111, &[(0, -20), (1, -40)]),
    );
    assert!(
        crate::session::hub_ref(&session)
            .resolved_represented_total_stat_multipliers_like_cpp()
            .is_none()
    );
    assert!(!session.send_stat_update());
    assert_eq!(effective_stats(&session), baseline);
}

#[tokio::test]
async fn fixed_ghost_and_wisp_auras_require_canonical_keys() {
    let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(43_975).await;
    let mut spells = wow_data::SpellStore::new();
    for spell_id in [
        wow_world_entities::SPELL_GHOST_LIKE_CPP,
        wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
    ] {
        spells.insert(
            spell_id,
            legacy_spell(spell_id, SPELL_AURA_MOD_PERCENT_STAT, -1, 0),
        );
    }
    session.set_spell_store(Arc::new(spells));
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([]),
    ));
    install_stats_catalog(&mut session);
    clear_auras(&mut session);
    assert!(session.send_stat_update());
    let baseline = effective_stats(&session);

    for (slot, spell_id) in [
        wow_world_entities::SPELL_GHOST_LIKE_CPP,
        wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
    ]
    .into_iter()
    .enumerate()
    {
        clear_auras(&mut session);
        insert_application(
            &mut session,
            application(spell_id, 0, slot as u8, 1, &[(0, -20)]),
        );
        assert!(!session.send_stat_update());
        assert_eq!(effective_stats(&session), baseline);
    }
}

#[tokio::test]
async fn missing_race_catalog_is_unknown_for_keyless_aura_classification() {
    let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(43_977).await;
    clear_auras(&mut session);
    install_stats_catalog(&mut session);
    let mut spells = wow_data::SpellStore::new();
    spells.insert(
        41_001,
        legacy_spell(41_001, SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE, -1, 0),
    );
    session.set_spell_store(Arc::new(spells));
    session.catalogs.chr.races_store = None;
    assert!(session.send_stat_update());
    let baseline = effective_stats(&session);

    insert_application(&mut session, application(41_001, 0, 4, 1, &[(0, -20)]));
    assert!(
        crate::session::hub_ref(&session)
            .resolved_represented_total_stat_multipliers_like_cpp()
            .is_none()
    );
    assert!(!session.send_stat_update());
    assert_eq!(effective_stats(&session), baseline);

    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([]),
    ));
    assert_eq!(
        crate::session::hub_ref(&session).resolved_represented_total_stat_multipliers_like_cpp(),
        Some([0.8; 5])
    );
    assert!(session.send_stat_update());
    assert_eq!(effective_stats(&session).stats, [80; 5]);
}

#[tokio::test]
async fn keyless_ordinary_grouped_stat_auras_keep_same_effect_arithmetic() {
    let (mut session, _send_rx, _realm_rx, _port) = logged_in_released_ghost(43_976).await;
    clear_auras(&mut session);
    install_stats_catalog(&mut session);
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([]),
    ));

    let mut spells = wow_data::SpellStore::new();
    for spell_id in [40_001, 40_002] {
        spells.insert(
            spell_id,
            legacy_spell(spell_id, SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE, -1, 0),
        );
    }
    session.set_spell_store(Arc::new(spells));
    let group_id = 1_001;
    let group = wow_data::SpellGroupStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::SpellGroupRowLikeCpp {
                group_id,
                spell_id: 40_001,
            },
            wow_data::SpellGroupRowLikeCpp {
                group_id,
                spell_id: 40_002,
            },
        ],
        |_| true,
        |_| 1,
    )
    .store;
    session.set_spell_group_store(Arc::new(group));
    let mut stack_rules = wow_data::SpellGroupStackRuleStoreLikeCpp::default();
    stack_rules.stack_rule_by_group_id.insert(
        group_id,
        wow_data::SpellGroupStackRuleLikeCpp::ExclusiveSameEffect,
    );
    stack_rules.same_effect_stack_by_group_id.insert(
        group_id,
        BTreeSet::from([SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE]),
    );
    session.set_spell_group_stack_rule_store(Arc::new(stack_rules));

    insert_application(&mut session, application(40_001, 0, 1, 1, &[(0, 20)]));
    insert_application(&mut session, application(40_002, 0, 2, 1, &[(0, 30)]));
    assert_eq!(
        crate::session::hub_ref(&session).resolved_represented_total_stat_multipliers_like_cpp(),
        Some([1.3; 5])
    );
}
