//! Mana, food, and drink recovery scenarios with their local fixture helpers.

use super::*;

#[test]
fn mana_regen_applies_canonical_aura_producers_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 86);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 77, 110);

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount, misc_value) in [
        (
            90_087,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_PCT,
            50,
            PowerType::Mana as i32,
        ),
        (
            90_088,
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
            10,
            PowerType::Mana as i32,
        ),
        (
            90_089,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_FROM_STAT,
            25,
            3,
        ),
        (
            90_090,
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_INTERRUPT,
            20,
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
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);
    for spell_id in [90_087, 90_088, 90_089, 90_090] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply mana regeneration aura");
    }

    let (_, changes) = session
        .player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        .expect("stat changes with mana regeneration aura");
    let spirit_regen = 40.0_f32.sqrt() * 30.0 * 0.003345;
    let expected_mp5 = 10.0 / 5.0 + 40.0 * 25.0 / 500.0;
    assert!((changes.mana_regen - (spirit_regen * 1.5 + expected_mp5)).abs() < 0.0001);
    assert!((changes.mana_regen_combat - (expected_mp5 + spirit_regen * 1.5 * 0.2)).abs() < 0.0001);
    assert_eq!(changes.mana_regen_mp5, 0.0);
}

fn publish_mana_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    mana_regen: f32,
    mana_regen_combat: f32,
) {
    let mut stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .unwrap_or_default();
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
fn mana_regeneration_tick_suppresses_then_publishes_power_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 87);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
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
    // The canonical snapshot is the sole source for the flat regen fields.
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 3.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // First second: the two-second publication boundary has not been reached,
    // so the value changes without an SMSG_POWER_UPDATE.
    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((110, 1_000))
    );
    assert!(
        !drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "throttled regeneration must not publish before 2000ms"
    );

    // Second second: the boundary is crossed and the packet is sent.
    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((120, 1_000))
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "crossing the 2000ms boundary publishes SMSG_POWER_UPDATE"
    );
}

#[test]
fn mana_regeneration_tick_uses_interrupted_rate_under_the_mp5_rule_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 88);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().world_mut().object_mut().add_to_world();
                // A cast paid mana one second ago, so the five-second rule is
                // active and the interrupted flat rate applies.
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session::game_time_ms_like_cpp().wrapping_sub(1_000),
                    );
            })
            .is_some()
    );
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 4.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        session.canonical_player_power_snapshot_like_cpp(PowerType::Mana),
        Some((104, 1_000)),
        "the interrupted flat modifier is consumed while the MP5 rule holds"
    );
}

/// Insert a one-effect aura spell and give its `SpellInfo` the supplied
/// `SpellAuraInterruptFlags` word, the input C++
/// `Player::RegenerateAll::findInterruptibleEffect` reads.
fn insert_food_emote_spell_like_cpp(
    store: &mut wow_data::SpellStore,
    spell_id: i32,
    aura_type: i32,
    amount: i32,
    misc_value: i32,
    aura_interrupt_flags: u32,
) {
    store.insert(
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
    store.insert_spell_interrupt_flags_like_cpp(spell_id, [aura_interrupt_flags, 0], [0, 0]);
}

/// Drain the realm copies C++ `Unit::SendPlaySpellVisualKit` publishes:
/// `(unit, kit_record_id, kit_type, duration)`.
fn drain_play_spell_visual_kits_like_cpp(
    send_rx: &flume::Receiver<Vec<u8>>,
) -> Vec<(ObjectGuid, i32, i32, u32)> {
    let mut kits = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let mut packet = WorldPacket::from_bytes(&bytes);
        if packet.server_opcode() != Some(wow_constants::ServerOpcodes::PlaySpellVisualKit) {
            continue;
        }
        let _opcode = packet.read_uint16().expect("PlaySpellVisualKit opcode");
        kits.push((
            packet.read_packed_guid().expect("visual unit"),
            packet.read_int32().expect("kit record"),
            packet.read_int32().expect("kit type"),
            packet.read_uint32().expect("kit duration"),
        ));
    }
    kits
}

fn food_emote_session_like_cpp(
    guid_entry: i64,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (mut session, send_rx) = make_session_with_send_capacity(32);
    let player_guid = ObjectGuid::create_player(1, guid_entry);
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
                player
                    .unit_mut()
                    .set_mp5_regeneration_interrupt_start_like_cpp(
                        crate::session::game_time_ms_like_cpp().wrapping_sub(10_000),
                    );
            })
            .is_some()
    );
    (session, send_rx, player_guid)
}

#[test]
fn food_emote_visual_prefers_standing_food_aura_like_cpp() {
    let (mut session, send_rx, player_guid) = food_emote_session_like_cpp(110);
    let mut spell_store = wow_data::SpellStore::new();
    // C++ prefers the food visual (`SPELL_VISUAL_KIT_FOOD`) when both a Standing
    // `SPELL_AURA_MOD_REGEN` and a Standing `SPELL_AURA_MOD_POWER_REGEN` apply.
    insert_food_emote_spell_like_cpp(
        &mut spell_store,
        90_110,
        wow_data::spell::aura_types::SPELL_AURA_MOD_REGEN,
        10,
        0,
        wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
    );
    insert_food_emote_spell_like_cpp(
        &mut spell_store,
        90_111,
        wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
        10,
        PowerType::Mana as i32,
        wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);
    for spell_id in [90_110, 90_111] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply food/drink aura");
    }
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // Four seconds stay below the independent five-second emote window.
    for _ in 0..4 {
        session.tick_player_regeneration_like_cpp(
            1_000,
            &power_types,
            None,
            &crate::PlayerRegenerationRatesLikeCpp::default(),
        );
    }
    assert!(
        drain_play_spell_visual_kits_like_cpp(&send_rx).is_empty(),
        "the food emote waits for its own five-second timer"
    );

    session.tick_player_regeneration_like_cpp(
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    let kits = drain_play_spell_visual_kits_like_cpp(&send_rx);
    assert!(
        kits.contains(&(player_guid, 406, 0, 0)),
        "crossing 5000ms sends SPELL_VISUAL_KIT_FOOD, got {kits:?}"
    );
    assert!(
        !kits.iter().any(|kit| kit.1 == 438),
        "the food visual wins over the drink visual"
    );
}

#[test]
fn drink_emote_visual_uses_standing_power_regen_aura_like_cpp() {
    let (mut session, send_rx, player_guid) = food_emote_session_like_cpp(111);
    let mut spell_store = wow_data::SpellStore::new();
    insert_food_emote_spell_like_cpp(
        &mut spell_store,
        90_112,
        wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
        10,
        PowerType::Mana as i32,
        wow_entities::SPELL_AURA_INTERRUPT_FLAG_STANDING_LIKE_CPP,
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .apply_aura(90_112, player_guid, 30_000, 1)
        .expect("apply drink aura");
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    for _ in 0..5 {
        session.tick_player_regeneration_like_cpp(
            1_000,
            &power_types,
            None,
            &crate::PlayerRegenerationRatesLikeCpp::default(),
        );
    }

    let kits = drain_play_spell_visual_kits_like_cpp(&send_rx);
    assert!(
        kits.contains(&(player_guid, 438, 0, 0)),
        "a Standing power-regen aura sends SPELL_VISUAL_KIT_DRINK, got {kits:?}"
    );
}

#[test]
fn food_emote_visual_skips_auras_without_standing_interrupt_flag_like_cpp() {
    let (mut session, send_rx, player_guid) = food_emote_session_like_cpp(112);
    let mut spell_store = wow_data::SpellStore::new();
    // A regeneration aura that is not interrupted by standing still never
    // produces the emote, and neither does a spell with no interrupt metadata.
    insert_food_emote_spell_like_cpp(
        &mut spell_store,
        90_113,
        wow_data::spell::aura_types::SPELL_AURA_MOD_REGEN,
        10,
        0,
        0,
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);
    session
        .apply_aura(90_113, player_guid, 30_000, 1)
        .expect("apply non-standing regen aura");
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    for _ in 0..6 {
        session.tick_player_regeneration_like_cpp(
            1_000,
            &power_types,
            None,
            &crate::PlayerRegenerationRatesLikeCpp::default(),
        );
    }

    assert!(
        drain_play_spell_visual_kits_like_cpp(&send_rx).is_empty(),
        "no Standing regen aura means no food/drink visual"
    );
}

#[test]
fn paying_a_mana_cost_arms_the_five_second_mp5_rule_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 89);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
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
        session
            .canonical_player_power_snapshot_like_cpp(PowerType::Mana)
            .map(|(current, _)| current),
        Some(450)
    );
    assert!(
        session.represented_player_mp5_regen_interrupted_like_cpp(),
        "Spell::TakePower arms the five-second MP5 rule after a mana cost"
    );
}
