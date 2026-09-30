//! Spell-granted extra-attack target scenarios.

use super::*;

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
