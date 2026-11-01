//! Scenarios for [`super`], part 13.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_melee_damage_delivery_filters_registry_state_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_057);
    let wrong_map = ObjectGuid::create_player(1, 58);
    let wrong_instance = ObjectGuid::create_player(1, 59);
    let not_in_world = ObjectGuid::create_player(1, 60);
    let missing = ObjectGuid::create_player(1, 61);
    let (wrong_map_info, wrong_map_rx) =
        make_registry_player_like_cpp(530, 0, Position::ZERO, true);
    let (wrong_instance_info, wrong_instance_rx) =
        make_registry_player_like_cpp(571, 9, Position::ZERO, true);
    let (not_in_world_info, not_in_world_rx) =
        make_registry_player_like_cpp(571, 0, Position::ZERO, false);
    registry.register_or_replace(wrong_map, wrong_map_info, Default::default());
    registry.register_or_replace(wrong_instance, wrong_instance_info, Default::default());
    registry.register_or_replace(not_in_world, not_in_world_info, Default::default());

    let make_command =
        |victim_guid| wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
            hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
            victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
            original_damage: 5,
            absorbed: 0,
            mana_spent: 0,
            absorb_consumptions: Vec::new(),
            split_combat_log_packets: Vec::new(),
            self_share_health_updates: Vec::new(),
        };
    let commands = vec![
        make_command(wrong_map),
        make_command(wrong_instance),
        make_command(not_in_world),
        make_command(missing),
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 4);
    assert_eq!(summary.candidates_seen, 3);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.candidates_skipped_wrong_map, 1);
    assert_eq!(summary.candidates_skipped_wrong_instance, 1);
    assert_eq!(summary.candidates_skipped_not_in_world, 1);
    assert_eq!(summary.candidates_skipped_missing_victim, 1);
    assert!(wrong_map_rx.try_recv().is_err());
    assert!(wrong_instance_rx.try_recv().is_err());
    assert!(not_in_world_rx.try_recv().is_err());
}
#[test]
fn creature_melee_damage_delivery_poisoned_durable_rail_counts_send_failed_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 62);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_058);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, _command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx, "MeleeFull");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    let durable = Arc::clone(&info.durable_creature_runtime_commands_like_cpp);
    let _ = std::thread::spawn(move || {
        let _guard = durable.lock().unwrap();
        panic!("poison durable rail for delivery failure coverage");
    })
    .join();
    registry.register_or_replace(victim, info, Default::default());

    let commands = vec![
        wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: attacker,
            victim_guid: victim,
            map_id: 571,
            instance_id: 0,
            damage: 5,
            over_damage: -1,
            target_level: 80,
            victim_health_after: 95,
            victim_health_state_revision_after: 1,
            hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
            victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
            original_damage: 5,
            absorbed: 0,
            mana_spent: 0,
            absorb_consumptions: Vec::new(),
            split_combat_log_packets: Vec::new(),
            self_share_health_updates: Vec::new(),
        },
    ];
    let summary = deliver_creature_melee_damage_commands_like_cpp(&commands, &registry);

    assert_eq!(summary.commands_seen, 1);
    assert_eq!(summary.candidates_seen, 1);
    assert_eq!(summary.candidates_queued, 0);
    assert_eq!(summary.send_failed, 1);
}
#[test]
fn creature_melee_damage_delivery_preserves_every_swing_when_general_queue_is_full_like_cpp() {
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let victim = ObjectGuid::create_player(1, 64);
    let attacker = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 9001, 90_063);
    let (send_tx, _send_rx) = flume::bounded::<Vec<u8>>(1);
    let (command_tx, command_rx) = flume::bounded::<SessionCommand>(1);
    let mut info = player_registration_fixture_like_cpp(send_tx, command_tx.clone(), "MeleeRetry");
    info.placement.map_id = 571;
    info.placement.instance_id = 0;
    info.placement.is_in_world = true;
    registry.register_or_replace(victim, info, Default::default());
    let command = wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        map_id: 571,
        instance_id: 0,
        damage: 5,
        over_damage: -1,
        target_level: 80,
        victim_health_after: 95,
        victim_health_state_revision_after: 7,
        hit_info: wow_packet::packets::combat::HIT_INFO_AFFECTS_VICTIM,
        victim_state: wow_packet::packets::combat::VICTIM_STATE_HIT,
        original_damage: 5,
        absorbed: 0,
        mana_spent: 0,
        absorb_consumptions: Vec::new(),
        split_combat_log_packets: Vec::new(),
        self_share_health_updates: Vec::new(),
    };
    command_tx
        .send(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(
            command.clone(),
        ))
        .unwrap();

    let mut latest = command.clone();
    latest.victim_health_after = 90;
    latest.victim_health_state_revision_after = 8;
    let summary = deliver_creature_melee_damage_commands_like_cpp(&[command, latest], &registry);
    assert_eq!(summary.candidates_queued, 2);
    assert_eq!(summary.send_failed, 0);
    assert_eq!(command_rx.len(), 1, "bounded general queue remains full");
    let commands = drain_durable_creature_runtime_commands_like_cpp(&registry, victim);
    assert_eq!(
        commands.len(),
        2,
        "every committed swing remains observable"
    );
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(first) = &commands[0] else {
        panic!("expected first durable melee command");
    };
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(second) = &commands[1] else {
        panic!("expected second durable melee command");
    };
    assert_eq!(first.victim_health_after, 95);
    assert_eq!(first.victim_health_state_revision_after, 7);
    assert_eq!(second.victim_health_after, 90);
    assert_eq!(second.victim_health_state_revision_after, 8);
}
