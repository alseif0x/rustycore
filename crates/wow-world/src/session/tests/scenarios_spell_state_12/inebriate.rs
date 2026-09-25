//! Player inebriate effect scenarios.

use super::*;

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
