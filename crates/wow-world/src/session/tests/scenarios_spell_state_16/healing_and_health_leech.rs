use super::*;

#[tokio::test]
async fn spell_heal_max_health_zero_damage_uses_caster_max_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 732_i32;
    let player_guid = ObjectGuid::create_player(1, 48);
    let creature_guid = test_creature_guid(18_014);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(65, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.take_damage(30);
            creature.creature.clear_data_changes();
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH,
                effect_base_points: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented heal-max-health row should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        world_creature.current_hp(),
        40,
        "C++ damage == 0 heals for caster max health, clamped by EffectHeal application"
    );
    drop(manager);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellHealLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}

#[tokio::test]
async fn spell_heal_pct_effect_row_heals_percent_of_target_max_health_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 734_i32;
    let player_guid = ObjectGuid::create_player(1, 51);
    let creature_guid = test_creature_guid(18_016);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(65, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.take_damage(30);
            creature.creature.clear_data_changes();
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented heal-pct row should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(
        world_creature.current_hp(),
        20,
        "C++ CountPctFromMaxHealth(25) heals 10 from a 40 max-health target"
    );
    drop(manager);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellHealLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}

#[tokio::test]
async fn spell_heal_pct_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 736_i32;
    let player_guid = ObjectGuid::create_player(1, 53);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(35, 80);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT,
            effect_base_points: -50,
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
        .expect("negative represented heal-pct should execute as C++ no-op effect");

    assert_eq!(session.player_health_like_cpp(), 35);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}

#[tokio::test]
async fn spell_health_leech_effect_row_damages_target_and_heals_caster_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 737_i32;
    let player_guid = ObjectGuid::create_player(1, 54);
    let creature_guid = test_creature_guid(18_035);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(50, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.take_damage(10);
            creature.creature.clear_data_changes();
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, creature_guid)
        .await
        .expect("represented health-leech row should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(world_creature.current_hp(), 5);
    drop(manager);
    assert_eq!(
        session.player_health_like_cpp(),
        75,
        "C++ HealthLeech heals the caster by the effective non-overkill damage"
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
async fn spell_health_leech_lethal_damage_heals_only_effective_damage_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 738_i32;
    let player_guid = ObjectGuid::create_player(1, 55);
    let creature_guid = test_creature_guid(18_036);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(50, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.take_damage(10);
            creature.creature.clear_data_changes();
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
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH,
            effect_base_points: 50,
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
        .expect("represented primary health-leech should execute");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(world_creature.current_hp(), 0);
    drop(manager);
    assert_eq!(
        session.player_health_like_cpp(),
        80,
        "C++ HealthLeech excludes overkill from the caster heal"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    // Lethal damage now has three C++ update-field effects bridged to the
    // client: `Player::SetXP`, the victim death state, and the caster heal.
    // The focused GiveXP test above validates the XP/level field mask;
    // this test keeps the health-leech ordering contract explicit.
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellNonMeleeDamageLog,
            ServerOpcodes::LogXpGain,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::SpellHealLog,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}

#[tokio::test]
async fn spell_health_leech_negative_amount_is_noop_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let manager = shared_map_manager();
    let spell_id = 739_i32;
    let player_guid = ObjectGuid::create_player(1, 56);
    let creature_guid = test_creature_guid(18_037);
    session.set_player_guid(Some(player_guid));
    session.set_player_health_like_cpp(50, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    register_test_creature(&mut session, manager.clone(), creature_guid, 40);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.take_damage(10);
            creature.creature.clear_data_changes();
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
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH,
            effect_base_points: -25,
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
        .expect("negative represented health-leech should execute as C++ no-op effect");

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, creature_guid).unwrap();
    assert_eq!(world_creature.current_hp(), 30);
    drop(manager);
    assert_eq!(session.player_health_like_cpp(), 50);
    let opcodes = drain_server_opcodes(&send_rx);
    assert_eq!(
        opcodes,
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
