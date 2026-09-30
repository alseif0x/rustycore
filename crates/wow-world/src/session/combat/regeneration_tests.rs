use std::sync::Arc;

use crate::session::WorldSession;
use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54_261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    (session, send_rx)
}

fn set_loaded_player_identity_like_cpp(
    session: &mut WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

fn attach_stat_update_player_with_mana_and_health(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
    current_health: u32,
    max_health: u32,
) {
    let identity = (
        session.player_map_id_like_cpp(),
        session.player_race_like_cpp(),
        session.player_class_like_cpp(),
        session.player_level_like_cpp(),
        session.player_gender_like_cpp(),
    );
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        session,
        u32::from(identity.0),
        0,
    );
    session.set_loaded_player_identity_like_cpp(
        identity.0, identity.1, identity.2, identity.3, identity.4,
    );
    let manager = session
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager fixture");
    assert!(crate::canonical_player_access::configure_canonical_player_vitals_for_test(
        manager,
        player_guid,
        (
            current_health,
            max_health,
            PowerType::Mana,
            current_mana,
            max_mana,
            0,
        ),
    ));
}

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
    assert!(session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(stats)
        })
        .is_some());
}

fn health_regen_game_tables_like_cpp(
    base_ratio: f32,
    more_ratio: f32,
) -> wow_data::RegenGameTablesLikeCpp {
    let mut base_columns = [0.0; wow_data::OctRegenHpGameTableLikeCpp::VALUE_COLUMN_COUNT];
    base_columns[4] = base_ratio;
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

fn mana_power_type_store_like_cpp(
    regen_peace: f32,
    regen_combat: f32,
) -> wow_data::character_progression::PowerTypeStore {
    wow_data::character_progression::PowerTypeStore::from_entries([
        wow_data::character_progression::PowerTypeEntry {
            id: 0,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum: PowerType::Mana as i8,
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

fn canonical_player_health_snapshot_for_test(
    session: &WorldSession,
) -> Option<(u32, u32)> {
    session.canonical_player_health_snapshot_like_cpp()
}

fn canonical_player_power_snapshot_for_test(
    session: &WorldSession,
    power_type: PowerType,
) -> Option<(i32, i32)> {
    session.canonical_player_power_snapshot_like_cpp(power_type)
}

fn tick_player_regeneration_for_test(
    session: &mut WorldSession,
    diff_ms: u32,
    power_types: &wow_data::character_progression::PowerTypeStore,
    regen_game_tables: Option<&wow_data::RegenGameTablesLikeCpp>,
    rates: &crate::PlayerRegenerationRatesLikeCpp,
) {
    session.tick_player_regeneration_like_cpp(diff_ms, power_types, regen_game_tables, rates);
}

#[test]
fn polymorph_transform_aura_allows_in_combat_health_regeneration_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 113);
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
    assert!(session
        .apply_aura_with_effect_mask_for_test_like_cpp(118, player_guid, 30_000, 0b11)
        .is_ok());
    assert_eq!(
        session.represented_player_is_polymorphed_like_cpp(),
        Some(true)
    );

    // C++ `Player::RegenerateHealth` (`Player.cpp:1857-1859`) replaces the
    // whole calculation with `GetMaxHealth() / 3.0f` while polymorphed, so the
    // in-combat gate that would otherwise suppress regeneration is bypassed.
    tick_player_regeneration_for_test(
        &mut session,
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
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
    tick_player_regeneration_for_test(
        &mut session,
        2_000,
        &power_types,
        Some(&tables),
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((433, 1_000)),
        "without the transform aura the in-combat gate suppresses regeneration"
    );
}

#[test]
fn paying_a_mana_cost_arms_the_five_second_mp5_rule_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 89);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 500, 1_000, 100, 100);
    // Keep the five-second rule inactive until the cast exercises the producer.
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session::game_time_ms_like_cpp().wrapping_sub(10_000),
                    );
            })
            .is_some()
    );
    assert!(!session.represented_player_mp5_regen_interrupted_like_cpp());

    let spell = wow_data::SpellInfo {
        spell_id: 90_091,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: vec![wow_data::SpellPowerCostInfoLikeCpp {
            order_index: 0,
            power_type: PowerType::Mana as i8,
            mana_cost: 50,
            mana_cost_per_level: 0,
            mana_per_second: 0,
            power_cost_pct: 0.0,
            power_cost_max_pct: 0.0,
            power_pct_per_second: 0.0,
            required_aura_spell_id: 0,
            optional_cost: 0,
        }],
        effects: Vec::new(),
    };
    let visual = wow_packet::packets::spell::SpellCastVisual {
        spell_visual_id: 0,
        script_visual_id: 0,
    };

    assert!(session.take_spell_power_like_cpp(&spell, ObjectGuid::EMPTY, spell.spell_id, &visual));
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana)
            .map(|(current, _)| current),
        Some(450)
    );
    assert!(
        session.represented_player_mp5_regen_interrupted_like_cpp(),
        "Spell::TakePower arms the five-second MP5 rule after a mana cost"
    );
}
