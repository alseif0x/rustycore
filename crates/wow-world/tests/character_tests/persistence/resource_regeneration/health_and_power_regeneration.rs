//! Health and non-mana power regeneration scenarios with local fixtures.

use super::*;

fn publish_health_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    spirit: i32,
    health_regen: i32,
) {
    assert!(
        mutate_canonical_player_for_test(
            session,
            |player| {
                let mut stats = *player.effective_combat_stats_like_cpp();
                stats.stats[4] = spirit;
                stats.health_regen = health_regen;
                player.replace_effective_combat_stats_like_cpp(stats);
            },
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
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
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
    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((100, 1_000)),
        "the health branch waits for the 2000ms window"
    );

    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((102, 1_000))
    );
    assert!(
        mutate_canonical_player_for_test(
            &session,
            |player| player
                .unit()
                .unit_data_changes_mask()
                .is_set(wow_entities::UNIT_DATA_HEALTH_BIT),
        )
        .unwrap_or(false),
        "the health write marks the UnitData field for the next VALUES update"
    );
}

#[test]
fn health_regeneration_tick_suppresses_in_combat_without_modifiers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 91);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
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

    tick_player_regeneration_for_test(
        &mut session,
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((100, 1_000)),
        "in combat without a during-combat aura there is no health regeneration"
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
        mutate_canonical_player_for_test(&session, |player| {
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
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
        })
        .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Rage, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    // C++ `PowerTypeEntry.RegenPeace` for rage is a per-second decay.
    let power_types = power_type_store_like_cpp(PowerType::Rage, -1.0, 0.0);

    tick_player_regeneration_for_test(
        &mut session,
        2_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Rage),
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
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 4, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
        })
        .is_some()
    );
    set_represented_primary_power_like_cpp(&mut session, PowerType::Energy, 50, 100);
    publish_health_regen_snapshot_like_cpp(&mut session, 0, 0);
    let power_types = power_type_store_like_cpp(PowerType::Energy, 10.0, 0.0);

    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Energy),
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
    assert!(
        mutate_canonical_player_for_test(
            session,
            |player| {
                let mut stats = *player.effective_combat_stats_like_cpp();
                stats.stats[4] = spirit;
                stats.health_regen = health_regen;
                stats.mana_regen = mana_regen;
                stats.mana_regen_combat = mana_regen_combat;
                player.replace_effective_combat_stats_like_cpp(stats);
            },
        )
        .is_some()
    );
}

#[test]
fn regeneration_rates_scale_mana_and_health_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 94);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(
        &mut session,
        player_guid,
        100,
        1_000,
        100,
        1_000,
    );
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
            // Keep the five-second rule inactive regardless of process uptime.
            player
                .unit_mut()
                .set_mp5_regeneration_interrupt_start_like_cpp(
                    game_time_ms_for_test().wrapping_sub(10_000),
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

    tick_player_regeneration_for_test(
        &mut session,
        2_000,
        &power_types,
        Some(&tables),
        &rates,
    );

    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((140, 1_000))
    );
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((104, 1_000))
    );
}
