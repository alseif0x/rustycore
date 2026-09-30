//! Preserved application scenarios for drain burn.

use super::*;

#[tokio::test]
async fn spell_power_drain_effect_drains_current_player_active_power_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 795_i32;
    let player_guid = ObjectGuid::create_player(1, 795);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
            15,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectPowerDrain should execute");

    let mana = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(mana, 10);
    assert_eq!(
        session.player_health_like_cpp(),
        100,
        "C++ self PowerDrain does not restore or damage the caster"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ],
        "C++ `ExecuteLogEffectTakeTargetPower` makes the cast publish its execute log"
    );
}

/// C++ `Spell::SendSpellExecuteLog` (`Spell.cpp:5048-5060`) carries the drained
/// power per effect: `PowerDrainTargets` rows are `Victim`, `uint32(Points)`,
/// `uint32(PowerType)` and `float(Amplitude)` (`CombatLogPackets.cpp:107-116`).
#[tokio::test]
async fn spell_power_drain_publishes_take_target_power_execute_log_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 795_i32;
    let player_guid = ObjectGuid::create_player(1, 795);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        15,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectPowerDrain should execute");

    let packets = drain_server_packet_bytes(&send_rx);
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellExecuteLog)
        })
        .expect("execute log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(log.read_int32().expect("spell id"), spell_id);
    assert_eq!(log.read_uint32().expect("effect count"), 1);
    assert_eq!(
        log.read_int32().expect("effect"),
        i32::try_from(wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN).unwrap()
    );
    assert_eq!(log.read_uint32().expect("power drain count"), 1);
    for _ in 0..5 {
        assert_eq!(log.read_uint32().expect("empty list count"), 0);
    }
    assert_eq!(log.read_packed_guid().expect("victim"), player_guid);
    assert_eq!(
        log.read_uint32().expect("points"),
        15,
        "C++ logs the drained power, not the remaining pool"
    );
    assert_eq!(
        log.read_uint32().expect("power type"),
        PowerType::Mana as u32
    );
    assert_eq!(
        log.read_float().expect("amplitude"),
        0.5,
        "C++ passes `CalcValueMultiplier` as the drain amplitude"
    );
    assert!(!log.has_bit().expect("has log data"));
    assert!(log.is_empty());
}
#[tokio::test]
async fn spell_power_drain_requires_active_power_type_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 796_i32;
    let player_guid = ObjectGuid::create_player(1, 796);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_display_power(PowerType::Energy);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
            15,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("mismatched represented EffectPowerDrain should be a no-op");

    let mana = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(mana, 25);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_power_drain_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 797_i32;
    let player_guid = ObjectGuid::create_player(1, 797);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
            -15,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("negative represented EffectPowerDrain should execute as C++ no-op effect");

    let mana = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(mana, 25);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_power_burn_effect_drains_power_and_damages_player_like_cpp_boundary() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 798_i32;
    let player_guid = ObjectGuid::create_player(1, 798);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
        15,
        PowerType::Mana,
    );
    // C++ `EffectPowerBurn` scales the drained power by
    // `CalcValueMultiplier` (`SpellEffects.cpp:1157-1164`).
    spell.effects[0].effect_amplitude = 1.0;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectPowerBurn should execute");

    let mana = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(mana, 10);
    assert_eq!(
        session.player_health_like_cpp(),
        85,
        "an amplitude of 1.0 burns `int32(15 * 1.0)` health"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ]
    );
}

/// C++ `EffectPowerBurn` logs the drained power with a zero amplitude
/// (`SpellEffects.cpp:1160`) and scales the health damage by
/// `SpellEffectInfo::CalcValueMultiplier` (`SpellEffects.cpp:1162`).
#[tokio::test]
async fn spell_power_burn_scales_damage_by_the_value_multiplier_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 798_i32;
    let player_guid = ObjectGuid::create_player(1, 798);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
        15,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 0.5;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectPowerBurn should execute");

    assert_eq!(
        session.player_health_like_cpp(),
        93,
        "int32(15 * 0.5) = 7 burned health"
    );

    let packets = drain_server_packet_bytes(&send_rx);
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellExecuteLog)
        })
        .expect("execute log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    log.read_packed_guid().expect("caster");
    log.read_int32().expect("spell id");
    log.read_uint32().expect("effect count");
    log.read_int32().expect("effect");
    log.read_uint32().expect("power drain count");
    for _ in 0..5 {
        log.read_uint32().expect("empty list count");
    }
    log.read_packed_guid().expect("victim");
    assert_eq!(log.read_uint32().expect("points"), 15);
    log.read_uint32().expect("power type");
    assert_eq!(
        log.read_float().expect("amplitude"),
        0.0,
        "C++ passes 0.0f for the burn amplitude even when the multiplier is non-zero"
    );
}
