// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! What the mailbox pump owns after #368: the order commands are presented in,
//! and the rule that an overflowed durable backlog disconnects the session
//! before any command is applied.
//!
//! Applying a command moved to `crate::session_commands`, so these are the
//! properties the kernel side must keep on its own.

use super::*;
use crate::session::mailbox::{
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand,
    MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP,
};

/// Draining is deliberately not "durable, then general": the durable rail is
/// presented only up to its first visibility-gated packet, so a refresh queued
/// on the general rail runs before it, and the durable remainder follows.
#[tokio::test]
async fn drain_presents_the_durable_prefix_then_the_general_rail_then_the_suffix_like_cpp() {
    let (session, _pkt_tx, _send_rx) = make_session();
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        9001,
        901,
    );
    let victim = ObjectGuid::create_player(1, 900);

    // Durable rail: one non-visibility command, then a visibility-gated one.
    {
        let mut pending = session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .expect("durable rail");
        assert!(
            pending.publish_attack_start_like_cpp(CreatureAttackStartLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                previous_victim_guid: None,
                map_id: 571,
                instance_id: 0,
                packet_already_broadcast: false,
            })
        );
        assert!(
            pending.publish_attack_stop_like_cpp(CreatureAttackStopLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 0,
            })
        );
    }

    // General rail: one command enqueued after both durable ones.
    session
        .session_command_tx()
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .expect("general rail accepts the command");

    let drained = session.drain_session_commands();
    let shapes: Vec<&'static str> = drained
        .iter()
        .map(|command| match command {
            SessionCommand::CreatureAttackStartLikeCpp(_) => "durable:start",
            SessionCommand::CreatureAttackStopLikeCpp(_) => "durable:stop",
            SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp => "general:refresh",
            _ => "other",
        })
        .collect();
    assert_eq!(
        shapes,
        vec!["durable:start", "durable:stop", "general:refresh"],
        "neither durable command is visibility-gated, so the whole rail precedes the general one"
    );
}

/// An overflowed durable backlog means authoritative transitions were lost.
/// The session is disconnected, and nothing queued behind the overflow is
/// applied — the kick happens before the drain, not after it.
#[tokio::test]
async fn an_overflowed_durable_backlog_kicks_before_any_command_is_applied_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);

    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        9001,
        903,
    );
    let overflowing = CreatureAttackStartLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: ObjectGuid::create_player(1, 902),
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    {
        let mut pending = session
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .expect("durable rail");
        for _ in 0..MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            assert!(pending.publish_attack_start_like_cpp(overflowing.clone()));
        }
        assert!(!pending.publish_attack_start_like_cpp(overflowing));
    }
    session
        .session_command_tx()
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .expect("general rail accepts the command");

    session
        .process_represented_session_commands_like_cpp()
        .await;

    assert!(
        session.is_disconnecting(),
        "an overflowed authoritative backlog must disconnect the desynchronized session"
    );
    assert_eq!(
        session.session_command_rx.len(),
        1,
        "the pump returns before draining, so the queued command is not applied"
    );
}
