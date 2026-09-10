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
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
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
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
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
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
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
        "represented boundary uses drained amount as PowerBurn damage until CalcValueMultiplier is ported"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
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
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
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
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
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
