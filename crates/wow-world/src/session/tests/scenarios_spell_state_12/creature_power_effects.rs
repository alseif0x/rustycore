//! Creature power-drain and power-burn scenarios.

use super::*;

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

/// C++ `Spell::EffectPowerBurn` (`SpellEffects.cpp:1157-1164`) multiplies the
/// drained power by `CalcValueMultiplier`, so a zero amplitude burns nothing
/// while the take-power row still logs the drained amount.
#[tokio::test]
async fn spell_power_burn_on_a_creature_with_zero_amplitude_deals_no_damage_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_303_i32;
    let player_guid = ObjectGuid::create_player(1, 906);
    let creature_guid = test_creature_guid(19_303);
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

    // `power_spell_info_like_cpp` leaves `effect_amplitude` at its 0.0 default.
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        power_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
            15,
            PowerType::Mana,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented creature EffectPowerBurn should execute");

    assert_eq!(
        session
            .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| creature
                .unit()
                .get_power(PowerType::Mana))
            .unwrap(),
        25,
        "the pool is burned regardless of the damage multiplier"
    );
    assert_eq!(
        manager
            .read()
            .unwrap()
            .find_creature(0, 7, creature_guid)
            .expect("creature in the test instance")
            .current_hp(),
        100,
        "C++ `int32(15 * 0.0)` burns no health"
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
    assert_eq!(log.read_uint32().expect("power drain count"), 1);
    for _ in 0..5 {
        log.read_uint32().expect("empty list count");
    }
    log.read_packed_guid().expect("victim");
    assert_eq!(
        log.read_uint32().expect("points"),
        15,
        "C++ logs the drained power before applying the zero multiplier"
    );
    log.read_uint32().expect("power type");
    assert_eq!(log.read_float().expect("amplitude"), 0.0);
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
