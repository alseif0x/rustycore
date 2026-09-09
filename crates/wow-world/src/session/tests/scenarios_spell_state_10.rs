//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_handle_spellclick_with_vehicle_seat_records_cpp_spellmod_values() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let vehicle_guid = test_creature_guid(229);
    let spell_id = 910_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 806,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 806,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_CONTROL_VEHICLE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![
                wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                    effect_aura: 0,
                    effect_base_points: 1,
                    ..Default::default()
                },
                wow_data::SpellEffectInfo {
                    effect_index: 1,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_CONTROL_VEHICLE,
                    effect_base_points: 0,
                    ..Default::default()
                },
            ],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    add_canonical_test_creature(
        &canonical,
        vehicle_guid,
        806,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    let plan =
        session.represented_handle_spell_click_plan_with_seat_like_cpp(vehicle_guid, Some(3));

    assert_eq!(plan.casts.len(), 1);
    assert_eq!(plan.casts[0].spell_id, u32::try_from(spell_id).unwrap());
    assert_eq!(plan.casts[0].vehicle_seat_id, Some(3));
    assert_eq!(plan.casts[0].vehicle_control_effect_index, Some(1));
    assert_eq!(
        plan.casts[0].vehicle_spellmod_basepoint_value,
        Some(4),
        "C++ CastSpell branch sets SPELLVALUE_BASE_POINT0 + effectIndex to seatId + 1"
    );
    assert_eq!(
        plan.casts[0].vehicle_aura_fallback_basepoint_value,
        Some(3),
        "C++ Aura::TryRefreshStackOrCreate fallback writes raw seatId into basepoints"
    );
}
#[test]
fn represented_handle_spellclick_with_vehicle_seat_skips_non_vehicle_aura_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let vehicle_guid = test_creature_guid(230);
    let spell_id = 911_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 807,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 807,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
                effect_aura: 0,
                effect_base_points: 7,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    add_canonical_test_creature(
        &canonical,
        vehicle_guid,
        807,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    let plan =
        session.represented_handle_spell_click_plan_with_seat_like_cpp(vehicle_guid, Some(2));

    assert!(plan.casts.is_empty());
    assert!(
        plan.ai_on_spell_click_unrepresented,
        "C++ still calls CreatureAI::OnSpellClick(clicker, false) after an invalid vehicle-enter aura row is skipped"
    );
}
#[test]
fn represented_handle_spellclick_continues_after_unrepresented_row_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(225);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 805,
                spell_id: 905,
                cast_flags: 0,
                user_type: SPELL_CLICK_USER_PARTY_LIKE_CPP,
            },
            wow_data::NpcSpellClickRowLikeCpp {
                npc_entry: 805,
                spell_id: 906,
                cast_flags: 0,
                user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
            },
        ],
        |entry| entry == 805,
        |spell| matches!(spell, 905 | 906),
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        805,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    assert!(plan.exact_context_unrepresented);
    assert_eq!(plan.casts.len(), 1);
    assert_eq!(plan.casts[0].spell_id, 906);
    assert_eq!(
        plan.casts[0].caster,
        RepresentedSpellClickUnitRefLikeCpp::Clickee
    );
    assert_eq!(
        plan.casts[0].target,
        RepresentedSpellClickUnitRefLikeCpp::Clickee
    );
    assert_eq!(
        plan.casts[0].original_caster,
        RepresentedSpellClickUnitRefLikeCpp::Clicker
    );
}
#[tokio::test]
async fn represented_spellclick_executes_clicker_cast_to_clickee_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(226);
    let spell_id = 907_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    // C++ always casts from a Player that is resident on the canonical map,
    // and login adopts its handle. The cast identity allocator fails closed
    // without both, so the fixture installs them like production does.
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.client_visible_guids_like_cpp.insert(creature_guid);
    session.set_map_manager(manager.clone());
    session.register_world_creature(
        571,
        Position::new(12.0, 0.0, 0.0, 0.0),
        test_creature_create_data(creature_guid, 9001, 40),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        9001,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9001,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9001,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;

    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: 1,
            executed_casts: 1,
            ai_on_spell_click_represented: true,
            ..Default::default()
        }
    );
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(571, 0, creature_guid).unwrap();
    assert_eq!(
        world_creature.current_hp(),
        33,
        "C++ Unit::HandleSpellClick casts the row spell when caster=clicker and target=clickee"
    );
    assert_eq!(
        world_creature
            .creature
            .ai_ownership()
            .last_spell_click_inform,
        Some(wow_entities::CreatureSpellClickInform {
            clicker: player_guid,
            spell_click_handled: true,
        }),
        "C++ calls CreatureAI::OnSpellClick(clicker, true) after a handled spellclick row"
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
async fn represented_spellclick_executes_clickee_caster_self_damage_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(227);
    let spell_id = 908_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    // C++ always casts from a Player that is resident on the canonical map,
    // and login adopts its handle. The cast identity allocator fails closed
    // without both, so the fixture installs them like production does.
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.client_visible_guids_like_cpp.insert(creature_guid);
    session.set_map_manager(manager.clone());
    session.register_world_creature(
        571,
        Position::new(12.0, 0.0, 0.0, 0.0),
        test_creature_create_data(creature_guid, 9002, 40),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        9002,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9002,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: 0,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9002,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;

    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: 1,
            executed_casts: 1,
            ai_on_spell_click_represented: true,
            ..Default::default()
        },
        "C++ Unit::HandleSpellClick may resolve caster=this and cast from the clickee itself"
    );
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(571, 0, creature_guid).unwrap();
    assert_eq!(world_creature.current_hp(), 33);
    assert_eq!(
        world_creature
            .creature
            .ai_ownership()
            .last_spell_click_inform,
        Some(wow_entities::CreatureSpellClickInform {
            clicker: player_guid,
            spell_click_handled: true,
        })
    );
    drop(manager);

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][0..2],
        &(ServerOpcodes::SpellGo as u16).to_le_bytes()
    );
    let mut spell_go = wow_packet::WorldPacket::from_bytes(&packets[0][2..]);
    assert_eq!(spell_go.read_packed_guid().unwrap(), creature_guid);
    assert_eq!(spell_go.read_packed_guid().unwrap(), creature_guid);
    assert_eq!(
        &packets[1][0..2],
        &(ServerOpcodes::UpdateObject as u16).to_le_bytes()
    );
}
#[tokio::test]
async fn represented_spellclick_executes_clickee_caster_damage_to_clicker_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(233);
    let spell_id = 914_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(50, 100);
    session.client_visible_guids_like_cpp.insert(creature_guid);
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        9008,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
    );
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    assert!(session.ensure_canonical_player_owner_for_map_like_cpp(
        wow_map::MapKey::new(571, 0),
        Position::new(10.0, 0.0, 0.0, 0.0),
    ));
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_max_health(100);
        player.unit_mut().set_health(50);
    });
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9008,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9008,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 9,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;

    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: 1,
            executed_casts: 1,
            ai_on_spell_click_represented: true,
            ..Default::default()
        },
        "C++ Unit::HandleSpellClick may resolve caster=this,target=clicker"
    );
    assert_eq!(session.player_health_like_cpp(), 41);
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player.unit().data().health)
            .unwrap(),
        41
    );
    assert_eq!(
        session
            .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
                creature.ai_ownership().last_spell_click_inform
            })
            .unwrap(),
        Some(wow_entities::CreatureSpellClickInform {
            clicker: player_guid,
            spell_click_handled: true,
        })
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][0..2],
        &(ServerOpcodes::SpellGo as u16).to_le_bytes()
    );
    let mut spell_go = wow_packet::WorldPacket::from_bytes(&packets[0][2..]);
    assert_eq!(spell_go.read_packed_guid().unwrap(), creature_guid);
    assert_eq!(spell_go.read_packed_guid().unwrap(), creature_guid);
    assert_eq!(
        &packets[1][0..2],
        &(ServerOpcodes::HealthUpdate as u16).to_le_bytes()
    );
}
#[tokio::test]
async fn represented_spellclick_executes_owner_original_caster_when_owner_is_clicker_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(229);
    let spell_id = 910_i32;

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    // C++ always casts from a Player that is resident on the canonical map,
    // and login adopts its handle. The cast identity allocator fails closed
    // without both, so the fixture installs them like production does.
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        0,
    );
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.client_visible_guids_like_cpp.insert(creature_guid);
    session.set_map_manager(manager.clone());
    session.register_world_creature(
        571,
        Position::new(12.0, 0.0, 0.0, 0.0),
        test_creature_create_data(creature_guid, 9004, 40),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    add_canonical_test_creature_on_map_with_world_state_and_owner(
        &canonical,
        creature_guid,
        9004,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
        571,
        0,
        true,
        Some(player_guid),
    );
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9004,
            spell_id: u32::try_from(spell_id).unwrap(),
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP
                | NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9004,
        |spell| spell == u32::try_from(spell_id).unwrap(),
    )));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: 7,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    assert_eq!(
        plan.casts[0].original_caster,
        RepresentedSpellClickUnitRefLikeCpp::Owner
    );
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;

    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: 1,
            executed_casts: 1,
            ai_on_spell_click_represented: true,
            ..Default::default()
        },
        "C++ GetOwnerGUID resolves to the clicker here, so Rust can use the player-caster rail without faking original caster"
    );
    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(571, 0, creature_guid).unwrap();
    assert_eq!(world_creature.current_hp(), 33);
    assert_eq!(
        world_creature
            .creature
            .ai_ownership()
            .last_spell_click_inform,
        Some(wow_entities::CreatureSpellClickInform {
            clicker: player_guid,
            spell_click_handled: true,
        })
    );
    drop(manager);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent
        ]
    );
}
#[tokio::test]
async fn represented_spellclick_skips_owner_original_caster_for_other_owner_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_owner_guid = ObjectGuid::create_player(1, 99);
    let creature_guid = test_creature_guid(230);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_creature_on_map_with_world_state_and_owner(
        &canonical,
        creature_guid,
        9005,
        Position::new(12.0, 0.0, 0.0, 0.0),
        UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32,
        571,
        0,
        true,
        Some(other_owner_guid),
    );
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [wow_data::NpcSpellClickRowLikeCpp {
            npc_entry: 9005,
            spell_id: 911,
            cast_flags: NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP
                | NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
            user_type: wow_data::SPELL_CLICK_USER_ANY_LIKE_CPP,
        }],
        |entry| entry == 9005,
        |spell| spell == 911,
    )));

    let plan = session.represented_handle_spell_click_plan_like_cpp(creature_guid);
    let outcome = session
        .execute_represented_spell_click_plan_like_cpp(creature_guid, &plan)
        .await;

    assert_eq!(
        outcome,
        RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: 1,
            ai_on_spell_click_unrepresented: true,
            skipped_unrepresented_original_caster: 1,
            ..Default::default()
        },
        "C++ would use the clickee owner as original caster; Rust must not collapse another owner into the clicker"
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn represented_unit_values_update_filters_spellclick_npc_flags_delta_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let creature_guid = test_creature_guid(125);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "Tester".to_string(),
        Position::new(10.0, 0.0, 0.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
    session.set_npc_spell_click_store(Arc::new(NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
        [],
        |entry| entry == 705,
        |_| false,
    )));
    add_canonical_test_creature(
        &canonical,
        creature_guid,
        705,
        Position::new(12.0, 0.0, 0.0, 0.0),
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
    );

    let mut mask = UpdateMask::new(UNIT_DATA_BITS);
    mask.set(113);
    mask.set(114);
    mask.set(115);
    let mut values = UnitDataValues::default();
    values.npc_flags = [
        (UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP as u32) | wow_constants::unit::NPCFlags1::GOSSIP.bits(),
        0xA5A5_0001,
    ];
    let update = UnitValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_UNIT,
        object_data: None,
        unit_data: Some(UnitDataUpdate { mask, values }),
    };

    let packet = session
        .represented_unit_values_update_to_update_object_like_cpp(creature_guid, 571, &update)
        .expect("delta packet");
    let wow_packet::packets::update::UpdateBlock::UnitValuesUpdate { data, .. } = &packet.blocks[0]
    else {
        panic!("unit values update block expected");
    };

    assert_eq!(
        data.npc_flags[0],
        wow_constants::unit::NPCFlags1::GOSSIP.bits()
    );
    assert_eq!(data.npc_flags[1], 0xA5A5_0001);
}
