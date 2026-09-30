//! Spell damage setup and base-calculation scenarios.

use super::*;

#[tokio::test]
async fn spell_effect_school_damage_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 723_i32;
    let guid = test_creature_guid(18_011);
    let player_guid = ObjectGuid::create_player(1, 54);
    session.player_guid = Some(player_guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: -10,
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
        .execute_spell(spell_id, guid)
        .await
        .expect("negative represented damage should execute as C++ no-op effect");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(world_creature.current_hp(), 40);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_direct_heal_and_damage_use_spell_effect_rows_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 724_i32;
    let guid = test_creature_guid(18_012);
    let player_guid = ObjectGuid::create_player(1, 55);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    register_test_creature(&mut session, manager.clone(), guid, 40);

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
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                    effect_base_points: 7,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
                    effect_base_points: 5,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented direct effects should execute per SpellEffectInfo row");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert_eq!(
        world_creature.current_hp(),
        38,
        "C++ HandleEffects executes damage and heal rows in effect order"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellNonMeleeDamageLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::SpellHealLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_school_damage_applies_spell_power_coefficient_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 725_i32;
    let guid = test_creature_guid(18_013);
    let player_guid = ObjectGuid::create_player(1, 56);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    // `GetBaseSpellPowerBonus()` 100 and the four stats the flat term can read.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    spell_power: 100,
                    stats: [10, 10, 10, 40, 30],
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            // Holy.
            school_mask: 1 << 1,
            ..Default::default()
        },
    ])));
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
            // `SpellEffectInfo::BonusCoefficient` in C++.
            effect_bonus_coefficient: 0.5,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented school damage should scale with spell power");

    let current_hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    // `int32(max((100 + int32(100 * 0.5)) * 1.0, 0))`.
    assert_eq!(
        current_hp, 850,
        "C++ SpellDamageBonusDone adds the advertised benefit times the coefficient"
    );
}

#[tokio::test]
async fn spell_school_damage_uses_max_damage_done_percent_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let spell_id = 726_i32;
    let guid = test_creature_guid(18_014);
    let player_guid = ObjectGuid::create_player(1, 57);
    session.player_guid = Some(player_guid);
    session.client_visible_guids_like_cpp.insert(guid);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 0, 0);
    register_test_creature(&mut session, manager.clone(), guid, 1_000);
    // Holy 1.5, fire 2.5: C++ takes the maximum over the spell's schools.
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(
                wow_entities::PlayerEffectiveCombatStatsLikeCpp {
                    mod_damage_done_percent: [1.0, 1.5, 2.5, 1.0, 1.0, 1.0, 1.0],
                    ..Default::default()
                },
            );
        })
        .expect("canonical player owner");

    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            // Holy | fire.
            school_mask: (1 << 1) | (1 << 2),
            ..Default::default()
        },
    ])));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, guid)
        .await
        .expect("represented school damage should use the school percentage");

    let current_hp = manager
        .read()
        .unwrap()
        .find_creature(0, 0, guid)
        .unwrap()
        .current_hp();
    // `SpellDamagePctDone`'s player branch takes `max(1.5, 2.5) = 2.5`.
    assert_eq!(current_hp, 750);
}
