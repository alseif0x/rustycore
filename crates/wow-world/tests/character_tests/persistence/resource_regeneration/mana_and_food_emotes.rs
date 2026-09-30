//! Mana, food, and drink recovery scenarios with their local fixture helpers.

use super::*;

fn publish_mana_regen_snapshot_like_cpp(
    session: &mut WorldSession,
    mana_regen: f32,
    mana_regen_combat: f32,
) {
    assert!(
        mutate_canonical_player_for_test(session, |player| {
            let mut stats = *player.effective_combat_stats_like_cpp();
            stats.mana_regen = mana_regen;
            stats.mana_regen_combat = mana_regen_combat;
            player.replace_effective_combat_stats_like_cpp(stats);
        })
        .is_some()
    );
}

#[test]
fn mana_regeneration_tick_suppresses_then_publishes_power_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(16);
    let player_guid = ObjectGuid::create_player(1, 87);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
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
    // The canonical snapshot is the sole source for the flat regen fields.
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 3.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    // First second: the two-second publication boundary has not been reached,
    // so the value changes without an SMSG_POWER_UPDATE.
    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((110, 1_000))
    );
    assert!(
        !drain_server_opcodes(&send_rx).contains(&wow_constants::ServerOpcodes::PowerUpdate),
        "throttled regeneration must not publish before 2000ms"
    );

    // Second second: the boundary is crossed and the packet is sent.
    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
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
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 100, 1_000, 100, 100);
    assert!(
        mutate_canonical_player_for_test(&session, |player| {
            player.unit_mut().world_mut().object_mut().add_to_world();
            // A cast paid mana one second ago, so the five-second rule is
            // active and the interrupted flat rate applies.
            player
                .unit_mut()
                .set_mp5_regeneration_interrupt_start_like_cpp(
                    game_time_ms_for_test().wrapping_sub(1_000),
                );
        })
        .is_some()
    );
    publish_mana_regen_snapshot_like_cpp(&mut session, 10.0, 4.0);
    let power_types = mana_power_type_store_like_cpp(0.0, 0.0);

    tick_player_regeneration_for_test(
        &mut session,
        1_000,
        &power_types,
        None,
        &crate::PlayerRegenerationRatesLikeCpp::default(),
    );

    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
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
            player
                .unit_mut()
                .set_mp5_regeneration_interrupt_start_like_cpp(
                    game_time_ms_for_test().wrapping_sub(10_000),
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
        tick_player_regeneration_for_test(
            &mut session,
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

    tick_player_regeneration_for_test(
        &mut session,
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
        tick_player_regeneration_for_test(
            &mut session,
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
        tick_player_regeneration_for_test(
            &mut session,
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
