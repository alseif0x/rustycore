//! Health and non-mana power regeneration scenarios with local fixtures.

use super::*;

fn publish_health_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    spirit: i32,
    health_regen: i32,
) {
    let mut stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap_or_default();
    stats.stats[4] = spirit;
    stats.health_regen = health_regen;
    assert!(
        session
            .mutate_canonical_player_like_cpp(
                |player| player.replace_effective_combat_stats_like_cpp(stats)
            )
            .is_some()
    );
}

/// Build `sOCTRegenHPGameTable` / `sRegenHPPerSptGameTable` level-80 Priest
/// rows and an empty `sRegenMPPerSptGameTable`.
fn health_regen_game_tables_like_cpp(
    base_ratio: f32,
    more_ratio: f32,
) -> wow_data::RegenGameTablesLikeCpp {
    let mut base_columns = [0.0; wow_data::OctRegenHpGameTableLikeCpp::VALUE_COLUMN_COUNT];
    base_columns[4] = base_ratio; // Priest column
    let mut more_columns = [0.0; wow_data::RegenHpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    more_columns[4] = more_ratio;
    let mut base_rows = vec![wow_data::OctRegenHpEntryLikeCpp::default(); 79];
    base_rows.push(wow_data::OctRegenHpEntryLikeCpp::from_columns(base_columns));
    let mut more_rows = vec![wow_data::RegenHpPerSptEntryLikeCpp::default(); 79];
    more_rows.push(wow_data::RegenHpPerSptEntryLikeCpp::from_columns(
        more_columns,
    ));
    wow_data::RegenGameTablesLikeCpp::from_tables(
        wow_data::RegenMpPerSptGameTableLikeCpp::from_rows([]),
        wow_data::RegenHpPerSptGameTableLikeCpp::from_rows(more_rows),
        wow_data::OctRegenHpGameTableLikeCpp::from_rows(base_rows),
    )
}

#[test]
fn health_regeneration_tick_heals_the_represented_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 90);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    // `OCTRegenHPPerSpirit` = Spirit(20) * 0.1 + 0 * 0.2 = 2.0.
    publish_health_regen_snapshot_like_cpp(&mut session, 20, 0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // C++ `RegenerateAll` only runs `RegenerateHealth` once the two-second
    // window is pending.
    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((100, 1_000)),
        "the health branch waits for the 2000ms window"
    );

    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((102, 1_000))
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| player
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_HEALTH_BIT))
            .unwrap_or(false),
        "the health write marks the UnitData field for the next VALUES update"
    );
}

#[test]
fn health_regeneration_tick_suppresses_in_combat_without_modifiers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 91);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                let mut flags = player.unit().unit_flags_like_cpp();
                flags.insert(wow_constants::unit::UnitFlags::IN_COMBAT);
                player.unit_mut().set_unit_flags_like_cpp(flags);
            })
            .is_some()
    );
    publish_health_regen_snapshot_like_cpp(&mut session, 20, 0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    session.tick_player_regeneration_like_cpp(
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((100, 1_000)),
        "in combat without a during-combat aura there is no health regeneration"
    );
}

#[test]
fn polymorph_transform_aura_allows_in_combat_health_regeneration_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 113);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                let mut flags = player.unit().unit_flags_like_cpp();
                flags.insert(wow_constants::unit::UnitFlags::IN_COMBAT);
                player.unit_mut().set_unit_flags_like_cpp(flags);
            })
            .is_some()
    );
    publish_health_regen_snapshot_like_cpp(&mut session, 20, 0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // C++ Polymorph (118): effect 0 applies `SPELL_AURA_MOD_CONFUSE` and the
    // `SPELL_AURA_TRANSFORM` effect owns `Unit::m_transformSpell`. The MAGE
    // class options (`SpellClassOptions.db2`) classify the spell specific as
    // `SPELL_SPECIFIC_MAGE_POLYMORPH`.
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        118,
        wow_data::SpellInfo {
            spell_id: 118,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([wow_data::SpellClassOptionsEntry {
            id: 1,
            spell_id: 118,
            modal_next_spell: 0,
            spell_class_set: 3,
            spell_class_mask: [0x0100_0000, 0, 0, 0],
        }]),
    ));
    session.set_state(crate::session::SessionState::LoggedIn);
    assert!(
        session
            .apply_aura_with_effect_mask_for_test_like_cpp(118, player_guid, 30_000, 0b11)
            .is_ok()
    );
    assert_eq!(
        session.represented_player_is_polymorphed_like_cpp(),
        Some(true)
    );

    // C++ `Player::RegenerateHealth` (`Player.cpp:1857-1859`) replaces the
    // whole calculation with `GetMaxHealth() / 3.0f` while polymorphed, so the
    // in-combat gate that would otherwise suppress regeneration is bypassed.
    session.tick_player_regeneration_like_cpp(
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((433, 1_000))
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(118)
        .expect("polymorph aura slot");
    session.remove_aura(slot).expect("remove polymorph aura");
    assert_eq!(
        session.represented_player_is_polymorphed_like_cpp(),
        Some(false)
    );
    session.tick_player_regeneration_like_cpp(
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((433, 1_000)),
        "without the transform aura the in-combat gate suppresses regeneration"
    );
}

fn power_type_store_like_cpp(
    power: PowerType,
    regen_peace: f32,
    regen_combat: f32,
) -> wow_data::character_progression::PowerTypeStore {
    wow_data::character_progression::PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 0,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: power as i8,
            min_power: 0,
            max_base_power: 0,
            center_power: 0,
            default_power: 0,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace,
            regen_combat,
            flags: 0,
        },
    ])
}

fn set_represented_primary_power_like_cpp(
    session: &mut WorldSession,
    power: PowerType,
    current: i32,
    max: i32,
) {
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                for raw in 0..wow_entities::MAX_POWERS as i8 {
                    if let Some(candidate) = <PowerType as num_traits::FromPrimitive>::from_i8(raw)
                    {
                        player.unit_mut().set_power_index(candidate, None);
                    }
                }
                player.unit_mut().set_power_index(power, Some(0));
                player.unit_mut().set_max_power(power, max);
                player.unit_mut().set_power(power, current);
            })
            .is_some()
    );
}

#[test]
fn non_mana_power_regeneration_tick_decays_rage_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 92);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Rage, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    // C++ `PowerTypeEntry.RegenPeace` for rage is a per-second decay.
    let power_types = power_type_store_like_cpp(PowerType::Rage, -1.0, 0.0);

    session.tick_player_regeneration_like_cpp(
        2_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Rage),
        Some((48, 100)),
        "the non-mana power loop decays the represented rage power"
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "crossing the two-second boundary publishes SMSG_POWER_UPDATE for the decayed power"
    );
}

#[test]
fn non_mana_power_regeneration_tick_throttles_energy_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 93);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 4, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
            })
            .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Energy, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    let power_types = power_type_store_like_cpp(PowerType::Energy, 10.0, 0.0);

    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Energy),
        Some((60, 100))
    );
    assert!(
        !drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "energy regeneration is throttled before the 2000ms boundary"
    );
}

fn publish_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    spirit: i32,
    health_regen: i32,
    mana_regen: f32,
    mana_regen_combat: f32,
) {
    let mut stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap_or_default();
    stats.stats[4] = spirit;
    stats.health_regen = health_regen;
    stats.mana_regen = mana_regen;
    stats.mana_regen_combat = mana_regen_combat;
    assert!(
        session
            .mutate_canonical_player_like_cpp(
                |player| player.replace_effective_combat_stats_like_cpp(stats)
            )
            .is_some()
    );
}

#[test]
fn regeneration_rates_scale_mana_and_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 94);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                // Keep the five-second rule inactive regardless of process uptime.
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session::game_time_ms_like_cpp().wrapping_sub(10_000),
                    );
            })
            .is_some()
    );
    // `OCTRegenHPPerSpirit` = 20 * 0.1 = 2.0; `Rate.Health = 2` doubles it and
    // `Rate.Mana = 2` doubles the published flat mana regeneration.
    publish_regen_snapshot_like_cpp(&mut session, 20, 0, 10.0, 0.0);
    let tables = health_regen_game_tables_like_cpp(0.1, 0.2);
    let rates = crate::PlayerRegenerationRatesLikeCpp {
        health: 2.0,
        mana: 2.0,
        ..crate::PlayerRegenerationRatesLikeCpp::default()
    };
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    session.tick_player_regeneration_like_cpp(2_000, &power_types, Some(&tables), &rates);

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((140, 1_000))
    );
    assert_eq!(
        session.canonical_player_health_snapshot_like_cpp(),
        Some((104, 1_000))
    );
}
