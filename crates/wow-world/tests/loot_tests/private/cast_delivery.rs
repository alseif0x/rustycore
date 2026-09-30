//! Original cast commit and visibility delivery regressions.

use super::recovery_support::*;
use wow_world::test_fixtures::loot::loot_visibility_matches_for_test;
use wow_constants::ServerOpcodes;
use wow_world::session::mailbox::{SendCreatureSpellCastIfVisibleLikeCppCommand, SendVisibleObjectValuesUpdateCommand};
use wow_world::test_fixtures::loot::{make_loot_cast_delivery_fixture, loot_committed_visibility_for_test, remove_loot_visibility_for_test, set_loot_combat_logging_for_test, drain_loot_delivery_commands_for_test, deliver_loot_visible_values_for_test, insert_loot_transport_visibility_for_test, clear_loot_transport_visibility_for_test};

fn creature_spell_cast_command_like_cpp(
    source_guid: ObjectGuid,
    committed_visibility_like_cpp: wow_world::session::mailbox::SharedClientVisibleGuidsLikeCpp,
    go_marker: u8,
) -> SendCreatureSpellCastIfVisibleLikeCppCommand {
    let mut start_packet_bytes = (ServerOpcodes::SpellStart as u16).to_le_bytes().to_vec();
    start_packet_bytes.push(0xAA);
    let mut go_packet_bytes = (ServerOpcodes::SpellGo as u16).to_le_bytes().to_vec();
    go_packet_bytes.push(go_marker);
    SendCreatureSpellCastIfVisibleLikeCppCommand {
        queued_at: Instant::now(),
        source_guid,
        map_id: 571,
        instance_id: 0,
        start_packet_bytes,
        go_packet_bytes,
        committed_visibility_like_cpp,
    }
}

#[tokio::test]
async fn advanced_combat_logging_receives_the_committed_full_creature_spell_go_like_cpp() {
    // The producer committed the full combat-log frame for this receiver.
    let (mut session, send_rx, source_guid) = make_loot_cast_delivery_fixture();
    set_loot_combat_logging_for_test(&mut session, true);
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        loot_committed_visibility_for_test(&session),
        0xCC,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    drain_loot_delivery_commands_for_test(&mut session)
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("full GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_cast_command_sends_start_then_basic_go_after_one_gate_like_cpp() {
    let (mut session, send_rx, source_guid) = make_loot_cast_delivery_fixture();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        loot_committed_visibility_for_test(&session),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    drain_loot_delivery_commands_for_test(&mut session)
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_cast_honors_commit_time_visibility_after_exit_like_cpp() {
    // C++ picks recipients synchronously inside `SendSpellGo`, so a
    // visibility exit between that commit and this drain cannot retract a
    // pair the viewer was already selected for.
    let (mut session, send_rx, source_guid) = make_loot_cast_delivery_fixture();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        loot_committed_visibility_for_test(&session),
        0xBB,
    );
    let expected_start = command.start_packet_bytes.clone();
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    assert!(
        remove_loot_visibility_for_test(&mut session, source_guid),
        "the caster leaves the client's visible set before the drain"
    );

    drain_loot_delivery_commands_for_test(&mut session)
        .await;

    assert_eq!(send_rx.try_recv().expect("START frame"), expected_start);
    assert_eq!(send_rx.try_recv().expect("GO frame"), expected_go);
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[tokio::test]
async fn creature_spell_cast_rejects_command_committed_for_another_session_like_cpp() {
    // A replaced session owns a fresh `HaveAtClient` allocation, so a pair
    // committed against the previous incarnation must not be delivered even
    // when the caster is visible again.
    let (mut session, send_rx, source_guid) = make_loot_cast_delivery_fixture();
    let previous_incarnation = wow_world::session::mailbox::SharedClientVisibleGuidsLikeCpp::default();
    previous_incarnation.insert(source_guid);
    assert!(
        !loot_visibility_matches_for_test(&session, &previous_incarnation),
        "the fixture models two distinct session incarnations"
    );
    let command = creature_spell_cast_command_like_cpp(source_guid, previous_incarnation, 0xBB);
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");

    drain_loot_delivery_commands_for_test(&mut session)
        .await;

    assert!(
        send_rx.try_recv().is_err(),
        "a command committed for another incarnation delivers nothing"
    );
}

#[tokio::test]
async fn creature_spell_go_keeps_the_committed_frame_after_a_preference_toggle_like_cpp() {
    // C++ chooses the combat-log representation while distributing the cast,
    // so toggling advanced logging before this drain must not retroactively
    // change the frame an earlier cast committed.
    let (mut session, send_rx, source_guid) = make_loot_cast_delivery_fixture();
    let command = creature_spell_cast_command_like_cpp(
        source_guid,
        loot_committed_visibility_for_test(&session),
        0xBB,
    );
    let expected_go = command.go_packet_bytes.clone();
    session
        .session_command_tx()
        .try_send(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
        .expect("atomic spell command queued");
    set_loot_combat_logging_for_test(&mut session, true);

    drain_loot_delivery_commands_for_test(&mut session)
        .await;

    let _start = send_rx.try_recv().expect("START frame");
    assert_eq!(
        send_rx.try_recv().expect("GO frame"),
        expected_go,
        "the committed basic frame survives a later advanced-logging toggle"
    );
    assert!(send_rx.try_recv().is_err(), "no partial or extra frame");
}

#[test]
fn transport_values_command_uses_visible_transport_membership_like_cpp() {
    let (mut session, send_rx, _) = make_loot_cast_delivery_fixture();
    let transport_guid = ObjectGuid::create_transport(HighGuid::Transport, 590003);
    let command = || SendVisibleObjectValuesUpdateCommand {
        object_guid: transport_guid,
        map_id: 571,
        packet_bytes: vec![0x52, 0x26],
        unit_values_update: None,
    };

    deliver_loot_visible_values_for_test(&mut session, command());
    assert!(send_rx.try_recv().is_err());
    insert_loot_transport_visibility_for_test(&mut session, transport_guid);
    deliver_loot_visible_values_for_test(&mut session, command());
    assert_eq!(send_rx.try_recv().unwrap(), vec![0x52, 0x26]);
    clear_loot_transport_visibility_for_test(&mut session);
    deliver_loot_visible_values_for_test(&mut session, command());
    assert!(send_rx.try_recv().is_err());
}
