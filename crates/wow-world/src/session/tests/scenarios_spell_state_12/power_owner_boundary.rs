//! Application boundaries around canonical power and charge transitions.
//! Original scenarios stay mounted unchanged; these cover operation failures
//! and the access/order distinctions not observable in a scalar math test.

use super::*;

fn interrupted_mana_store() -> wow_data::character_progression::PowerTypeStore {
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
            flags: 0x0002,
        },
    ])
}

#[test]
fn energize_interrupt_packet_precedes_nonpositive_max_refusal() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_001);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    session.set_power_type_store(Arc::new(interrupted_mana_store()));
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_max_power(PowerType::Mana, 0);
    }).unwrap();

    assert!(!session.apply_energize_effect_like_cpp(1, guid, 20, 0, guid, false));
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::InterruptPowerRegen]);
    assert_eq!(session.mutate_canonical_player_like_cpp(|player|
        player.get_power(PowerType::Mana)).unwrap(), 0);
}

#[test]
fn energize_admission_refuses_bad_power_and_other_targets_before_interrupt() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_002);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    session.set_power_type_store(Arc::new(interrupted_mana_store()));
    for power in [-1, MAX_POWERS as i32] {
        assert!(!session.apply_energize_effect_like_cpp(1, guid, 20, power, guid, false));
    }
    assert!(!session.apply_energize_effect_like_cpp(
        1, guid, 20, 0, ObjectGuid::create_player(1, 98_003), false,
    ));
    assert!(drain_server_opcodes(&send_rx).is_empty());
    assert_eq!(session.mutate_canonical_player_like_cpp(|player|
        player.get_power(PowerType::Mana)).unwrap(), 25);
}

#[test]
fn loaded_canonical_charge_rounds_share_history_but_report_max_not_sum() {
    let (mut session, _, send_rx) = make_session();
    let guid = crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session, 0, 0,
    );
    session.with_owned_player_mut_like_cpp(|player| {
        let history = &mut player.unit_mut().subsystems_mut().spells.history;
        history.charges_loaded = true;
        for index in 0..4 {
            history.add_charge_state_like_cpp(7, index * 10, (index + 1) * 10);
        }
    }).unwrap();
    assert_eq!(session.apply_modify_spell_charges_effect_like_cpp(2, 7, guid), 2);
    assert!(!session.player_spell_history_snapshot_like_cpp().unwrap().charges.contains_key(&7));
    assert!(drain_server_opcodes(&send_rx).is_empty(), "charge restoration has no new wire producer");
}

#[test]
fn failed_canonical_charge_round_still_runs_each_loaded_fixture_iteration() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_004);
    session.set_player_guid(Some(guid));
    for index in 0..3 {
        session.record_loaded_character_spell_charge_like_cpp(7, 100 + index, 200 + index);
    }
    session.mark_represented_character_spell_charges_loaded_like_cpp();

    assert_eq!(session.apply_modify_spell_charges_effect_like_cpp(2, 7, guid), 2);
    let history = session.player_spell_history_snapshot_like_cpp().unwrap();
    assert_eq!(history.charges[&7].iter().copied().collect::<Vec<_>>(), vec![
        wow_entities::SpellChargeState { recharge_start_ms: 100_000, recharge_end_ms: 200_000 },
    ]);
    assert_eq!(session.apply_modify_spell_charges_effect_like_cpp(2, 7, guid), 1);
    assert!(!session.player_spell_history_snapshot_like_cpp().unwrap().charges.contains_key(&7));
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn stale_handle_never_enables_charge_or_power_fixture_fallback() {
    let (mut session, _, send_rx) = make_session();
    let guid = crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session, 0, 0,
    );
    session.with_owned_player_mut_like_cpp(|player| {
        let history = &mut player.unit_mut().subsystems_mut().spells.history;
        history.charges_loaded = true;
        history.add_charge_state_like_cpp(7, 10, 20);
    }).unwrap();
    let original_manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let handle = session.player_handle_like_cpp.unwrap();
    session.set_canonical_map_manager(Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    session.set_power_type_store(Arc::new(interrupted_mana_store()));

    assert_eq!(session.apply_modify_spell_charges_effect_like_cpp(1, 7, guid), 0);
    assert!(!session.apply_energize_effect_like_cpp(1, guid, 20, 0, guid, false));
    assert!(session.player_spell_history_snapshot_like_cpp().is_none());
    assert_eq!(original_manager.lock().unwrap().with_player_like_cpp(handle, |player|
        player.unit().subsystems().spells.history.consumed_charges(7)), Some(1));
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[tokio::test]
async fn drain_preserves_world_active_power_mapping_for_ids_twenty_through_twenty_five() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_005);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    for power in [
        PowerType::RuneBlood, PowerType::RuneFrost, PowerType::RuneUnholy,
        PowerType::AlternateQuest, PowerType::AlternateEncounter, PowerType::AlternateMount,
    ] {
        session.mutate_canonical_player_like_cpp(|player| {
            let unit = player.unit_mut();
            unit.set_power_index(power, Some(0));
            unit.set_max_power(power, 100);
            unit.set_power(power, 30);
            unit.set_display_power(power);
        }).unwrap();
        assert!(session.apply_power_drain_effect_like_cpp(
            &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
            10, power as i32, guid, false, 1.0, ObjectGuid::EMPTY, 0,
        ).await);
        assert_eq!(session.mutate_canonical_player_like_cpp(|player|
            player.get_power(power)).unwrap(), 20);
    }
    assert_eq!(session.spell_state.represented_spell_execute_log_effects_like_cpp[0]
        .power_drain_targets.len(), 6);
    assert!(drain_server_opcodes(&send_rx).is_empty(), "self drain has no caster energize log");
}

#[tokio::test]
async fn empty_player_drain_returns_false_but_retains_its_execute_log() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_006);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_power(PowerType::Mana, 0);
        player.unit_mut().set_display_power(PowerType::Mana);
    }).unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    assert!(!session.apply_power_drain_effect_like_cpp(
        &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        10, 0, guid, false, 0.5, ObjectGuid::EMPTY, 0,
    ).await);
    let row = &session.spell_state.represented_spell_execute_log_effects_like_cpp[0].power_drain_targets[0];
    assert_eq!((row.victim, row.points, row.power_type, row.amplitude), (guid, 0, 0, 0.5));
    session.send_spell_execute_log_like_cpp(guid, 1);
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::SpellExecuteLog]);
}

#[tokio::test]
async fn creature_burn_keeps_power_and_log_when_the_damage_owner_is_missing() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_007);
    let creature_guid = test_creature_guid(98_007);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    let canonical = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical, creature_guid, 9_001, Position::new(10.0, 20.0, 30.0, 0.0), 0, 0, 80,
    );
    session.mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
        let unit = creature.unit_mut();
        unit.set_power_index(PowerType::Mana, Some(0));
        unit.set_max_power(PowerType::Mana, 100);
        unit.set_power(PowerType::Mana, 40);
        unit.set_display_power(PowerType::Mana);
        creature.clear_data_changes();
    }).unwrap();
    assert!(session.map_manager.is_none(), "the separate damage owner is deliberately absent");
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    assert!(session.apply_power_drain_effect_like_cpp(
        &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
        10, 0, creature_guid, true, 2.0, ObjectGuid::EMPTY, 0,
    ).await);
    assert_eq!(session.mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature|
        (creature.unit().get_power(PowerType::Mana), creature.unit().data().health)).unwrap(),
        (30, 100));
    let row = &session.spell_state.represented_spell_execute_log_effects_like_cpp[0].power_drain_targets[0];
    assert_eq!((row.victim, row.points, row.amplitude), (creature_guid, 10, 2.0));
    assert!(drain_server_opcodes(&send_rx).is_empty(), "no visibility update or damage log was admitted");
    session.send_spell_execute_log_like_cpp(guid, 1);
    assert_eq!(drain_server_opcodes(&send_rx), vec![ServerOpcodes::SpellExecuteLog]);
}

#[tokio::test]
async fn creature_drain_fails_closed_when_two_fallback_maps_are_ambiguous() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_008);
    let creature_guid = test_creature_guid(98_008);
    session.set_player_guid(Some(guid));
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    for instance in [0, 7] {
        add_canonical_test_creature_indexed_on_map_with_level(
            &canonical, creature_guid, 9_001, Position::new(10.0, 20.0, 30.0, 0.0), 0, instance, 80,
        );
        let mut manager = canonical.lock().unwrap();
        let creature = manager.find_map_mut(0, instance).unwrap()
            .map_mut().get_typed_creature_mut(creature_guid).unwrap();
        creature.unit_mut().set_power_index(PowerType::Mana, Some(0));
        creature.unit_mut().set_max_power(PowerType::Mana, 100);
        creature.unit_mut().set_power(PowerType::Mana, 40);
        creature.unit_mut().set_display_power(PowerType::Mana);
        creature.clear_data_changes();
    }
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    assert!(!session.apply_power_drain_effect_like_cpp(
        &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        10, 0, creature_guid, false, 1.0, ObjectGuid::EMPTY, 0,
    ).await);
    let manager = canonical.lock().unwrap();
    for instance in [0, 7] {
        assert_eq!(manager.find_map(0, instance).unwrap().map()
            .get_typed_creature(creature_guid).unwrap().unit().get_power(PowerType::Mana), 40);
    }
    assert!(session.spell_state.represented_spell_execute_log_effects_like_cpp.is_empty());
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[tokio::test]
async fn empty_creature_drain_returns_true_and_publishes_zero_caster_gain() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_009);
    let creature_guid = test_creature_guid(98_009);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    let canonical = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical, creature_guid, 9_001, Position::new(10.0, 20.0, 30.0, 0.0), 0, 0, 80,
    );
    session.mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
        creature.unit_mut().set_power_index(PowerType::Mana, Some(0));
        creature.unit_mut().set_max_power(PowerType::Mana, 100);
        creature.unit_mut().set_power(PowerType::Mana, 0);
        creature.unit_mut().set_display_power(PowerType::Mana);
        creature.clear_data_changes();
    }).unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    assert!(session.apply_power_drain_effect_like_cpp(
        &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN,
        10, 0, creature_guid, false, 0.5, ObjectGuid::EMPTY, 0,
    ).await);
    assert_eq!(session.mutate_canonical_player_like_cpp(|player|
        player.get_power(PowerType::Mana)).unwrap(), 25);
    let row = &session.spell_state.represented_spell_execute_log_effects_like_cpp[0].power_drain_targets[0];
    assert_eq!((row.victim, row.points, row.amplitude), (creature_guid, 0, 0.5));
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(packet.server_opcode(), Some(ServerOpcodes::SpellEnergizeLog));
    packet.read_uint16().unwrap();
    assert_eq!(packet.read_packed_guid().unwrap(), guid);
    assert_eq!(packet.read_packed_guid().unwrap(), guid);
    assert_eq!(packet.read_int32().unwrap(), 1);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
    assert_eq!(packet.read_int32().unwrap(), 0);
}

#[tokio::test]
async fn player_burn_keeps_nan_and_negative_multiplier_casts_and_zero_log_amplitude() {
    let (mut session, _, send_rx) = make_session();
    let guid = ObjectGuid::create_player(1, 98_010);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, guid, 100, 100);
    session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_power(PowerType::Mana, 30);
        player.unit_mut().set_display_power(PowerType::Mana);
    }).unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(HighGuid::Item, 1);
    for multiplier in [f32::NAN, -0.5] {
        assert!(session.apply_power_drain_effect_like_cpp(
            &generator, 1, wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN,
            10, 0, guid, true, multiplier, ObjectGuid::EMPTY, 0,
        ).await);
    }
    assert_eq!(session.mutate_canonical_player_like_cpp(|player|
        (player.get_power(PowerType::Mana), player.unit().data().health)).unwrap(), (10, 100));
    let rows = &session.spell_state.represented_spell_execute_log_effects_like_cpp[0].power_drain_targets;
    assert_eq!(rows.iter().map(|row| (row.points, row.amplitude)).collect::<Vec<_>>(),
        vec![(10, 0.0), (10, 0.0)]);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
