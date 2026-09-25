//! Spell reputation-effect scenarios.

use super::*;

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
