//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

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
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
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
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
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
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
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
            ServerOpcodes::LogXpGain,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
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
#[tokio::test]
async fn spell_kill_credit_effect_row_rewards_player_monster_objective_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 740_i32;
    let player_guid = ObjectGuid::create_player(1, 57);
    let quest_id = 12_540;
    let creature_entry = 9_940;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 0, // C++ QUEST_OBJECTIVE_MONSTER.
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT,
                effect_misc_value_1: creature_entry as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented kill-credit spell row should execute");

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_kill_credit2_effect_row_rewards_current_session_like_cpp_without_group_fanout() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 741_i32;
    let player_guid = ObjectGuid::create_player(1, 58);
    let quest_id = 12_541;
    let creature_entry = 9_941;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 0, // C++ QUEST_OBJECTIVE_MONSTER.
        order: 0,
        storage_index: 0,
        object_id: creature_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2,
                effect_misc_value_1: creature_entry as i32,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented kill-credit2 spell row should execute");

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_give_honor_effect_row_sends_pvp_credit_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 747_i32;
    let player_guid = ObjectGuid::create_player(1, 64);
    session.set_player_guid(Some(player_guid));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented give-honor spell row should execute");

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::PvpCredit,
            ServerOpcodes::CooldownEvent,
        ]
    );
    let mut credit = wow_packet::WorldPacket::from_bytes(&packets[1]);
    assert_eq!(
        credit.read_uint16().expect("opcode"),
        ServerOpcodes::PvpCredit as u16
    );
    assert_eq!(credit.read_int32().expect("OriginalHonor"), 25);
    assert_eq!(credit.read_int32().expect("Honor"), 25);
    assert_eq!(
        credit.read_packed_guid().expect("Target"),
        ObjectGuid::EMPTY,
        "C++ EffectGiveHonor leaves PvPCredit.Target default-initialized"
    );
    assert_eq!(credit.read_int32().expect("Rank"), 0);
    assert!(credit.is_empty());
}
#[tokio::test]
async fn spell_give_honor_effect_row_adds_honor_xp_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 749_i32;
    let player_guid = ObjectGuid::create_player(1, 67);
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Honor".to_string(),
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();

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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 8_825,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented give-honor spell row should add honor XP");

    assert_eq!(
        session.mutate_canonical_player_like_cpp(|player| {
            (
                player.data().honor_level,
                player.active_data().honor,
                player.active_data().honor_next_level,
            )
        }),
        Some((1, 25, 8_800))
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::PvpCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_give_honor_effect_row_requires_current_player_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 748_i32;
    let player_guid = ObjectGuid::create_player(1, 65);
    let other_player_guid = ObjectGuid::create_player(1, 66);
    session.set_player_guid(Some(player_guid));
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR,
                effect_base_points: 25,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_player_guid)
        .await
        .expect("represented give-honor non-current player target should no-op");

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_dismiss_pet_effect_row_clears_represented_pet_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 753_i32;
    let player_guid = ObjectGuid::create_player(1, 72);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 73);
    session.set_player_guid(Some(player_guid));
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
    );
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, pet_guid)
        .await
        .expect("represented dismiss-pet spell row should execute");

    assert_eq!(session.represented_pet_guid_like_cpp, None);
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_dismiss_pet_effect_row_requires_represented_pet_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 754_i32;
    let player_guid = ObjectGuid::create_player(1, 74);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 75);
    let other_pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 76);
    session.set_player_guid(Some(player_guid));
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
    );
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, other_pet_guid)
        .await
        .expect("represented dismiss-pet non-active pet target should no-op");

    assert_eq!(session.represented_pet_guid_like_cpp, Some(pet_guid));
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
