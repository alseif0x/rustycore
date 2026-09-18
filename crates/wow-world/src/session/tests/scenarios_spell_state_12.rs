//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_spell_positivity_covers_common_cpp_buffs_and_debuffs() {
    let mut periodic_heal = threat_spell_info_like_cpp(
        18_146,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        10,
    );
    periodic_heal.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_PERIODIC_HEAL;
    periodic_heal.effects[0].implicit_target_1 = 21; // TARGET_UNIT_TARGET_ALLY
    assert!(crate::session_rules::represented_spell_is_positive_like_cpp(&periodic_heal));

    let mut absorb = periodic_heal.clone();
    absorb.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB;
    assert!(crate::session_rules::represented_spell_is_positive_like_cpp(&absorb));

    let mut stat_buff = periodic_heal.clone();
    stat_buff.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_STAT;
    assert!(crate::session_rules::represented_spell_is_positive_like_cpp(&stat_buff));
    stat_buff.effects[0].effect_base_points = -10;
    assert!(!crate::session_rules::represented_spell_is_positive_like_cpp(&stat_buff));

    let mut enemy_periodic_damage = periodic_heal;
    enemy_periodic_damage.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE;
    enemy_periodic_damage.effects[0].implicit_target_1 = 6; // TARGET_UNIT_TARGET_ENEMY
    assert!(!crate::session_rules::represented_spell_is_positive_like_cpp(&enemy_periodic_damage));
}
#[tokio::test]
async fn spell_energize_effect_restores_current_player_power_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 793_i32;
    let player_guid = ObjectGuid::create_player(1, 793);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            50,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectEnergize should execute");

    let mana = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(mana, 75);
    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .map(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes)
                .server_opcode()
                .expect("server opcode")
        })
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellEnergizeLog,
            ServerOpcodes::CooldownEvent
        ]
    );
    // C++ `Unit::SendEnergizeSpellLog` (`Unit.cpp:6566-6576`): the target and
    // caster GUIDs, the spell, the `Powers` value, the applied delta and the
    // amount the pool could not take.
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellEnergizeLog)
        })
        .expect("energize log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("target"), player_guid);
    assert_eq!(log.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(log.read_int32().expect("spell id"), spell_id);
    assert_eq!(
        log.read_int32().expect("power type"),
        PowerType::Mana as i32
    );
    assert_eq!(log.read_int32().expect("amount"), 50);
    assert_eq!(log.read_int32().expect("over energize"), 0);
    assert!(!log.has_bit().expect("has log data"));
    assert!(log.is_empty());
}

/// C++ `Unit::EnergizeBySpell` (`Unit.cpp:6578-6590`): `gain` is what
/// `ModifyPower` actually applied, so a pool with less room than the requested
/// amount reports the applied delta and the `OverEnergize` remainder.
#[tokio::test]
async fn spell_energize_reports_the_applied_delta_and_over_energize_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 796_i32;
    let player_guid = ObjectGuid::create_player(1, 796);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            let max = player.get_max_power(PowerType::Mana);
            player.unit_mut().set_power(PowerType::Mana, max - 10);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            50,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectEnergize should execute");

    let packets = drain_server_packet_bytes(&send_rx);
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellEnergizeLog)
        })
        .expect("energize log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    log.read_packed_guid().expect("target");
    log.read_packed_guid().expect("caster");
    assert_eq!(log.read_int32().expect("spell id"), spell_id);
    assert_eq!(
        log.read_int32().expect("power type"),
        PowerType::Mana as i32
    );
    assert_eq!(log.read_int32().expect("amount"), 10);
    assert_eq!(log.read_int32().expect("over energize"), 40);
}
#[tokio::test]
async fn spell_energize_pct_effect_uses_target_max_power_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 794_i32;
    let player_guid = ObjectGuid::create_player(1, 794);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT,
            25,
            PowerType::Energy,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectEnergizePct should execute");

    let energy = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Energy))
        .unwrap();
    assert_eq!(energy, 55);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellEnergizeLog,
            ServerOpcodes::CooldownEvent
        ]
    );
}
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
#[tokio::test]
async fn spell_add_extra_attacks_effect_records_selected_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 799_i32;
    let player_guid = ObjectGuid::create_player(1, 799);
    let selected_target = test_creature_guid(18_799);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_target(selected_target);
        })
        .unwrap();

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS,
                effect_base_points: 2,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectAddExtraAttacks should execute");

    let extra_attacks = session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit().extra_attacks_for_like_cpp(selected_target)
        })
        .unwrap();
    assert_eq!(extra_attacks, 2);

    // C++ `ExecuteLogEffectExtraAttacks` (`Spell.cpp:5088-5095`) writes the
    // victim and `uint32(NumAttacks)` into the effect's `ExtraAttacksTargets`.
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                wow_packet::WorldPacket::from_bytes(bytes)
                    .server_opcode()
                    .expect("server opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ]
    );
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
        i32::try_from(wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS).unwrap()
    );
    assert_eq!(log.read_uint32().expect("power drain count"), 0);
    assert_eq!(log.read_uint32().expect("extra attacks count"), 1);
    for _ in 0..4 {
        assert_eq!(log.read_uint32().expect("empty list count"), 0);
    }
    assert_eq!(
        log.read_packed_guid().expect("victim"),
        player_guid,
        "C++ `ExecuteLogEffectExtraAttacks` logs the spell's `unitTarget`, which is the self-cast caster"
    );
    assert_eq!(log.read_uint32().expect("num attacks"), 2);
}
#[tokio::test]
async fn spell_add_extra_attacks_prefers_last_damaged_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 800_i32;
    let player_guid = ObjectGuid::create_player(1, 800);
    let selected_target = test_creature_guid(18_800);
    let last_damaged_target = test_creature_guid(18_801);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_target(selected_target);
            player
                .unit_mut()
                .set_last_damaged_target_like_cpp(Some(last_damaged_target));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS,
            effect_base_points: 3,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary EffectAddExtraAttacks should execute");

    let (selected_extra, last_damaged_extra) = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().extra_attacks_for_like_cpp(selected_target),
                player
                    .unit()
                    .extra_attacks_for_like_cpp(last_damaged_target),
            )
        })
        .unwrap();
    assert_eq!(selected_extra, 0);
    assert_eq!(last_damaged_extra, 3);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn spell_add_extra_attacks_without_target_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 801_i32;
    let player_guid = ObjectGuid::create_player(1, 801);
    let selected_target = test_creature_guid(18_802);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS,
            effect_base_points: 2,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("target-less represented EffectAddExtraAttacks should execute as C++ no-op");

    let extra_attacks = session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit().extra_attacks_for_like_cpp(selected_target)
        })
        .unwrap();
    assert_eq!(extra_attacks, 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_inebriate_effect_increases_current_player_drunk_value_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 802_i32;
    let player_guid = ObjectGuid::create_player(1, 802);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(15);
        })
        .unwrap();

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
                effect_base_points: 35,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 50);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_inebriate_effect_clamps_to_hundred_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 803_i32;
    let player_guid = ObjectGuid::create_player(1, 803);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(90);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: 30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented primary EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 100);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_inebriate_effect_negative_amount_sobers_and_clamps_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 804_i32;
    let player_guid = ObjectGuid::create_player(1, 804);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(20);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: -30,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("negative represented EffectInebriate should execute");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 0);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_inebriate_effect_ignores_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 805_i32;
    let player_guid = ObjectGuid::create_player(1, 805);
    let creature_guid = test_creature_guid(18_805);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    register_test_creature(&mut session, shared_map_manager(), creature_guid, 40);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.set_inebriation_like_cpp(10);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE,
            effect_base_points: 40,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("non-player target represented EffectInebriate should execute as C++ no-op");

    let inebriation = session
        .mutate_canonical_player_like_cpp(|player| player.inebriation_like_cpp())
        .unwrap();
    assert_eq!(inebriation, 10);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_reputation_effect_modifies_current_player_reputation_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 806_i32;
    let faction_id = 7_u32;
    let rep_list_id = 5_u32;
    let player_guid = ObjectGuid::create_player(1, 806);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
                effect_base_points: 250,
                effect_misc_value_1: faction_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectReputation should execute");

    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| {
            mgr.get_state(rep_list_id).map(|state| state.standing)
        }),
        Some(Some(250))
    );

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SetFactionStanding,
            ServerOpcodes::CooldownEvent
        ]
    );

    let packet = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SetFactionStanding)
        })
        .expect("set faction standing packet");
    let mut reader = wow_packet::WorldPacket::from_bytes(packet);
    reader.skip_opcode();
    assert_eq!(reader.read_float().unwrap(), 0.0);
    assert_eq!(reader.read_uint32().unwrap(), 1);
    assert_eq!(reader.read_int32().unwrap(), rep_list_id as i32);
    assert_eq!(reader.read_int32().unwrap(), 250);
    assert!(!reader.read_bit().unwrap());
}
#[tokio::test]
async fn spell_reputation_effect_uses_spell_reward_rate_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let spell_id = 807_i32;
    let faction_id = 7_u32;
    let rep_list_id = 5_u32;
    let player_guid = ObjectGuid::create_player(1, 807);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([faction]);
    let (reward_rate_store, report) =
        wow_data::reputation::ReputationRewardRateStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::ReputationRewardRateRowLikeCpp {
                faction_id,
                rates: wow_data::reputation::ReputationRewardRateEntryLikeCpp {
                    quest_rate: 1.0,
                    quest_daily_rate: 1.0,
                    quest_weekly_rate: 1.0,
                    quest_monthly_rate: 1.0,
                    quest_repeatable_rate: 1.0,
                    creature_rate: 1.0,
                    spell_rate: 1.5,
                },
            }],
            &faction_store,
        );
    assert_eq!(report.loaded, 1);
    session.set_faction_store(Arc::new(faction_store));
    session.set_reputation_reward_rate_store(Arc::new(reward_rate_store));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
            effect_base_points: 200,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
                effect_base_points: 200,
                effect_misc_value_1: faction_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectReputation should execute");

    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| {
            mgr.get_state(rep_list_id).map(|state| state.standing)
        }),
        Some(Some(300))
    );
}
#[tokio::test]
async fn spell_reputation_effect_ignores_missing_faction_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 808_i32;
    let player_guid = ObjectGuid::create_player(1, 808);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session.set_faction_store(Arc::new(FactionStore::from_entries([])));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
            effect_base_points: 250,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
                effect_base_points: 250,
                effect_misc_value_1: 7,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("missing faction represented EffectReputation should execute as C++ no-op");

    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| mgr.get_state(5).is_none()),
        Some(true)
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_reputation_effect_ignores_non_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 809_i32;
    let faction_id = 7_u32;
    let rep_list_id = 5_u32;
    let player_guid = ObjectGuid::create_player(1, 809);
    let creature_guid = test_creature_guid(18_809);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    register_test_creature(&mut session, shared_map_manager(), creature_guid, 40);
    let mut faction = FactionEntry::for_test_like_cpp(faction_id, rep_list_id as i16);
    faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    session.set_faction_store(Arc::new(FactionStore::from_entries([faction])));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
            effect_base_points: 250,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION,
                effect_base_points: 250,
                effect_misc_value_1: faction_id as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("non-player target represented EffectReputation should execute as C++ no-op");

    assert_eq!(
        session.with_reputation_mgr_like_cpp(|mgr| {
            mgr.get_state(rep_list_id).map(|state| state.standing)
        }),
        Some(Some(0))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

/// C++ `Spell::EffectEnergize` level-dependent cases
/// (`SpellEffects.cpp:1507-1524`): Blood Fury subtracts
/// `10 * max(0, min(30, level - 60))` and Burst of Energy
/// `4 * max(0, min(15, level - 60))` before `EnergizeBySpell`.
#[tokio::test]
async fn spell_energize_level_dependent_spells_scale_the_amount_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1_245);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);

    let blood_fury = 24_571_i32;
    let burst_of_energy = 24_532_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        blood_fury,
        power_spell_info_like_cpp(
            blood_fury,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            300,
            PowerType::Mana,
        ),
    );
    spell_store.insert(
        burst_of_energy,
        power_spell_info_like_cpp(
            burst_of_energy,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            100,
            PowerType::Energy,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(blood_fury, player_guid)
        .await
        .expect("represented Blood Fury should execute");
    session
        .execute_spell(burst_of_energy, player_guid)
        .await
        .expect("represented Burst of Energy should execute");

    // Level 80: mana gains 300 - 10 * min(30, 20) = 100; energy gains
    // 100 - 4 * min(15, 20) = 40.
    let (mana, energy) = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Energy),
            )
        })
        .unwrap();
    assert_eq!(mana, 125);
    assert_eq!(energy, 70);
}

/// C++ `Spell::EffectEnergize` Runic Mana Injector case
/// (`SpellEffects.cpp:1518-1524`): a caster Player with `SKILL_ENGINEERING`
/// gains an extra `AddPct(damage, 25)`.
#[tokio::test]
async fn spell_energize_runic_mana_injector_engineering_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1_246);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_max_power(PowerType::Mana, 600);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();

    let injector = 67_490_i32;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        injector,
        power_spell_info_like_cpp(
            injector,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            100,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(injector, player_guid)
        .await
        .expect("represented Runic Mana Injector should execute");
    let without_skill = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(
        without_skill, 125,
        "C++ `HasSkill(SKILL_ENGINEERING)` is false without a skill record"
    );

    session.set_represented_player_skill_like_cpp(202, 1, 1, 450);
    session
        .execute_spell(injector, player_guid)
        .await
        .expect("represented Runic Mana Injector should execute for an engineer");
    let with_skill = session
        .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
        .unwrap();
    assert_eq!(
        with_skill, 250,
        "C++ `AddPct(damage, 25)` turns 100 into 125 for an engineer"
    );
}

/// C++ `Unit::EnergizeBySpell` (`Unit.cpp:6586`) forwards `damage / 2`
/// assisting threat with `ignoreModifiers = true`, so the spell's threat
/// percentage does not modify the forwarded amount.
#[tokio::test]
async fn spell_energize_forwards_half_requested_threat_without_modifiers_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 1_247);
    let creature_guid = test_creature_guid(18_247);
    let energize_spell_id = 18_247;
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Energizer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    {
        let mut legacy = manager.write().unwrap();
        let creature = legacy
            .remove_creature_any(0, 0, creature_guid)
            .expect("move the represented creature into the test instance");
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        legacy.add_creature(0, 7, grid_x, grid_y, creature);
    }
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player_guid);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .add_threat(player_guid, 10.0);
        })
        .unwrap();
    session.sync_represented_creature_threat_to_canonical_like_cpp(
        creature_guid,
        player_guid,
        10.0,
    );
    // A threat entry would double the forwarded amount when modifiers applied.
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: HashMap::from([(
            energize_spell_id as u32,
            wow_data::SpellThreatEntryLikeCpp {
                flat_mod: 4,
                pct_mod: 2.0,
                ap_pct_mod: 0.0,
            },
        )]),
    }));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        energize_spell_id,
        power_spell_info_like_cpp(
            energize_spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
            50,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(energize_spell_id, player_guid)
        .await
        .expect("represented energize should execute");

    let threat = session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .unit()
                .subsystems()
                .combat
                .threat_value(player_guid)
        })
        .flatten()
        .expect("the threatening creature records the forwarded threat");
    // 10 existing threat + the cast-level positive `HandleThreatSpells` bonus
    // (flat 4 * pctMod 2.0 = 8) + the `EnergizeBySpell` forwarding
    // (50 / 2 = 25, `ignoreModifiers = true`), which the 2.0 percentage must
    // not double to 50.
    assert_eq!(threat, 43.0);
    assert_eq!(
        session.canonical_creature_threat_value_like_cpp(creature_guid, player_guid),
        Some(43.0)
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        75
    );
}

/// C++ `Unit::EnergizeBySpell` (`Unit.cpp:6581-6585`) → `Player::InterruptPowerRegen`
/// (`Player.cpp:1831-1840`): a power whose DB2 entry carries
/// `PowerTypeFlags::UseRegenInterrupt` publishes `SMSG_INTERRUPT_POWER_REGEN`
/// before the energize log; a power without the flag publishes nothing.
#[tokio::test]
async fn spell_energize_interrupts_flagged_power_regen_like_cpp() {
    fn power_type_store(
        use_regen_interrupt: bool,
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
                regen_interrupt_time_ms: 5_000,
                regen_peace: 0.0,
                regen_combat: 0.0,
                flags: if use_regen_interrupt { 0x0002 } else { 0 },
            },
        ])
    }

    for (use_regen_interrupt, expected) in [
        (
            true,
            vec![
                ServerOpcodes::SpellGo,
                ServerOpcodes::InterruptPowerRegen,
                ServerOpcodes::SpellEnergizeLog,
                ServerOpcodes::CooldownEvent,
            ],
        ),
        (
            false,
            vec![
                ServerOpcodes::SpellGo,
                ServerOpcodes::SpellEnergizeLog,
                ServerOpcodes::CooldownEvent,
            ],
        ),
    ] {
        let (mut session, _, send_rx) = make_session();
        let spell_id = 797_i32;
        let player_guid = ObjectGuid::create_player(1, 797);
        configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
        session.set_power_type_store(Arc::new(power_type_store(use_regen_interrupt)));

        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            power_spell_info_like_cpp(
                spell_id,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE,
                50,
                PowerType::Mana,
            ),
        );
        session.set_spell_store(Arc::new(spell_store));

        session
            .execute_spell(spell_id, player_guid)
            .await
            .expect("represented EffectEnergize should execute");

        assert_eq!(
            drain_server_opcodes(&send_rx),
            expected,
            "C++ `Player::InterruptPowerRegen` fires only for `UseRegenInterrupt` powers, before `SendEnergizeSpellLog`"
        );
        assert_eq!(
            session
                .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
                .unwrap(),
            75
        );
    }
}

/// C++ `Spell::EffectPowerDrain` returns before touching the pool when the
/// target's `GetPowerType()` differs from the effect's power
/// (`SpellEffects.cpp:1078`).
#[tokio::test]
async fn spell_power_drain_on_a_creature_requires_the_matching_power_type_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_302_i32;
    let player_guid = ObjectGuid::create_player(1, 905);
    let creature_guid = test_creature_guid(19_302);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Drainer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();
    session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let unit = creature.unit_mut();
            unit.set_power_index(PowerType::Mana, Some(0));
            unit.set_max_power(PowerType::Mana, 100);
            unit.set_power(PowerType::Mana, 40);
            // The creature's active power type is Energy, not the drained Mana.
            unit.set_display_power(PowerType::Energy);
            creature.clear_data_changes();
        })
        .unwrap();

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
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("a mismatched represented creature EffectPowerDrain is a C++ no-op");

    assert_eq!(
        session
            .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| creature
                .unit()
                .get_power(PowerType::Mana))
            .unwrap(),
        40,
        "C++ leaves the pool untouched when GetPowerType() differs"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        25,
        "no caster share is restored for a refused drain"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent],
        "a refused drain publishes neither the energize log nor an execute log"
    );
}

/// C++ `Spell::EffectPowerDrain` (`SpellEffects.cpp:1069-1102`) drains any living
/// target whose `GetPowerType()` matches the effect and restores
/// `drained * CalcValueMultiplier` to the caster through `EnergizeBySpell`.
#[tokio::test]
async fn spell_power_drain_on_a_creature_restores_the_caster_share_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_300_i32;
    let player_guid = ObjectGuid::create_player(1, 903);
    let creature_guid = test_creature_guid(19_300);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Drainer".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();
    session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let unit = creature.unit_mut();
            unit.set_power_index(PowerType::Mana, Some(0));
            unit.set_max_power(PowerType::Mana, 100);
            unit.set_power(PowerType::Mana, 40);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

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
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature EffectPowerDrain should execute");

    assert_eq!(
        session
            .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| creature
                .unit()
                .get_power(PowerType::Mana))
            .unwrap(),
        25,
        "C++ drains the target's pool by the effect amount"
    );
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.get_power(PowerType::Mana))
            .unwrap(),
        32,
        "C++ `EnergizeBySpell` restores int32(15 * 0.5) = 7 to the caster"
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                wow_packet::WorldPacket::from_bytes(bytes)
                    .server_opcode()
                    .expect("server opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellEnergizeLog,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ],
        "the caster gain publishes its log from the effect, the take-power row with the finished cast"
    );
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
    assert_eq!(log.read_packed_guid().expect("victim"), creature_guid);
    assert_eq!(log.read_uint32().expect("points"), 15);
    assert_eq!(
        log.read_uint32().expect("power type"),
        PowerType::Mana as u32
    );
    assert_eq!(log.read_float().expect("amplitude"), 0.5);
}

/// C++ `Spell::EffectPowerBurn` (`SpellEffects.cpp:1142-1165`) drains the
/// target's power and adds `int32(drained * CalcValueMultiplier)` to the
/// spell's damage; for a creature target the represented damage path applies it.
#[tokio::test]
async fn spell_power_burn_on_a_creature_applies_the_scaled_damage_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_301_i32;
    let player_guid = ObjectGuid::create_player(1, 904);
    let creature_guid = test_creature_guid(19_301);
    let position = Position::new(10.0, 20.0, 30.0, 0.0);
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Burner".to_string(),
        position,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 100);
    add_canonical_test_player_on_map(&canonical, player_guid, position, 0, 7);
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        creature_guid,
        9_001,
        position,
        0,
        7,
        80,
    );
    {
        let mut legacy = manager.write().unwrap();
        let creature = legacy
            .remove_creature_any(0, 0, creature_guid)
            .expect("move the represented creature into the test instance");
        let (grid_x, grid_y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        legacy.add_creature(0, 7, grid_x, grid_y, creature);
    }
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_power_index(PowerType::Mana, Some(0));
            player.unit_mut().set_max_power(PowerType::Mana, 200);
            player.unit_mut().set_power(PowerType::Mana, 25);
            player.clear_data_changes();
        })
        .unwrap();
    session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            let unit = creature.unit_mut();
            unit.set_health(100);
            unit.set_power_index(PowerType::Mana, Some(0));
            unit.set_max_power(PowerType::Mana, 100);
            unit.set_power(PowerType::Mana, 40);
            unit.set_display_power(PowerType::Mana);
            creature.clear_data_changes();
        })
        .unwrap();

    let mut spell = power_spell_info_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
        15,
        PowerType::Mana,
    );
    spell.effects[0].effect_amplitude = 1.0;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, spell);
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature EffectPowerBurn should execute");

    let mana = session
        .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
            creature.unit().get_power(PowerType::Mana)
        })
        .unwrap();
    assert_eq!(mana, 25, "C++ drains the burned power from the target pool");
    let health = manager
        .read()
        .unwrap()
        .find_creature(0, 7, creature_guid)
        .expect("creature in the test instance")
        .current_hp();
    assert_eq!(
        health, 85,
        "C++ adds int32(15 * 1.0) to the spell damage, applied by the represented creature damage path"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::UpdateObject),
        "the drained power is a unit data field and is published to the visible client: {opcodes:?}"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::SpellExecuteLog),
        "the take-power row still ships with the finished cast: {opcodes:?}"
    );
}
