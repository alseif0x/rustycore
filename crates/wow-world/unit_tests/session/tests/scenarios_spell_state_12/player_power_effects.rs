//! Player energize, drain, burn, and power side-effect scenarios.

use super::*;

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
