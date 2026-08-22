// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session mailbox tests for [`super`].
//!
//! Moved verbatim from `wow_network::player_registry` by issue #140. Following
//! the extraction convention of issue #214, the only textual change is
//! dedenting by one level.

#![cfg(test)]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use wow_core::ObjectGuid;

use super::*;

/// A producer selecting recipients must never observe a visibility
/// transition half applied, nor the old membership while the packets that
/// replace it are already queued.
#[test]
fn visibility_transition_publishes_membership_and_packets_as_one_step_like_cpp() {
    use std::sync::mpsc;
    use std::time::Duration;

    let visibility = SharedClientVisibleGuidsLikeCpp::default();
    let leaving = ObjectGuid::create_player(1, 1);
    let arriving = ObjectGuid::create_player(1, 2);
    visibility.insert(leaving);

    let reader_handle = visibility.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (observed_tx, observed_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        entered_rx
            .recv()
            .expect("transition entered its publish step");
        // This read can only complete once the transition released its write.
        observed_tx
            .send(reader_handle.snapshot_like_cpp())
            .expect("reader reported its snapshot");
    });

    visibility.publish_transition_like_cpp(
        |guid| *guid != leaving,
        [arriving],
        || {
            entered_tx.send(()).expect("reader was signalled");
            std::thread::sleep(Duration::from_millis(50));
            assert!(
                observed_rx.try_recv().is_err(),
                "no reader may observe the set while its packets are being published"
            );
        },
    );

    let observed = observed_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("reader snapshot");
    reader.join().expect("reader finished");
    assert!(
        !observed.contains(&leaving),
        "the departed object is gone once the transition is observable"
    );
    assert!(
        observed.contains(&arriving),
        "the arrived object is present once the transition is observable"
    );
}

/// Verify that `PlayerBroadcastInfo` carries `instance_id` so that

/// Verify that `SendIfVisibleLikeCppCommand` carries both `map_id` and
/// `instance_id` — required so per-session gate can reject cross-instance
/// delivery (Slice 4A.1b).
#[test]
fn send_if_visible_like_cpp_command_carries_map_and_instance_id() {
    let guid = ObjectGuid::create_player(1, 7);
    let cmd = SendIfVisibleLikeCppCommand {
        queued_at: Instant::now(),
        source_guid: guid,
        map_id: 532,
        instance_id: 99,
        packet_bytes: vec![0xDE, 0xAD],
    };
    assert_eq!(cmd.map_id, 532);
    assert_eq!(cmd.instance_id, 99);
}

#[test]
fn creature_spell_start_and_committed_go_use_one_durable_queue_element_like_cpp() {
    let source_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 458);
    let command = SendCreatureSpellCastIfVisibleLikeCppCommand {
        queued_at: Instant::now(),
        source_guid,
        map_id: 571,
        instance_id: 4,
        start_packet_bytes: vec![0x37, 0x2C, 0xAA],
        go_packet_bytes: vec![0x36, 0x2C, 0xBB],
        committed_visibility_like_cpp: SharedClientVisibleGuidsLikeCpp::default(),
    };
    let mut durable = DurableCreatureRuntimeCommandsLikeCpp::default();

    assert!(durable.publish_creature_spell_cast_if_visible_like_cpp(command.clone()));
    let drained = durable.drain_like_cpp();

    assert_eq!(
        drained.len(),
        1,
        "a drain cannot split START from GO because the pair occupies one FIFO element"
    );
    let [SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(drained_command)] =
        drained.as_slice()
    else {
        panic!("expected one atomic spell cast command: {drained:?}");
    };
    assert_eq!(drained_command, &command);
}

/// Verify that creature visibility refresh commands are scoped by both map
/// and instance. The receiving `WorldSession` applies the same gates before
/// forcing its visibility pass.
#[test]
fn refresh_visible_world_creatures_like_cpp_command_carries_map_and_instance_id() {
    let cmd = RefreshVisibleWorldCreaturesLikeCppCommand {
        map_id: 571,
        instance_id: 7,
    };
    assert_eq!(cmd.map_id, 571);
    assert_eq!(cmd.instance_id, 7);
}

/// Verify the creature melee damage command carries both addressing data
/// and final victim health so session delivery can be idempotent.
#[test]
fn apply_creature_melee_damage_like_cpp_command_carries_final_health() {
    let attacker =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 456);
    let victim = ObjectGuid::create_player(1, 7);
    let cmd = ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        map_id: 571,
        instance_id: 3,
        damage: 11,
        over_damage: -1,
        target_level: 80,
        victim_health_after: 89,
        victim_health_state_revision_after: 7,
    };

    assert_eq!(cmd.attacker_guid, attacker);
    assert_eq!(cmd.victim_guid, victim);
    assert_eq!(cmd.map_id, 571);
    assert_eq!(cmd.instance_id, 3);
    assert_eq!(cmd.victim_health_after, 89);
    assert_eq!(cmd.victim_health_state_revision_after, 7);
}

#[test]
fn creature_attack_start_like_cpp_command_carries_map_and_instance_id() {
    let attacker =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 123, 457);
    let victim = ObjectGuid::create_player(1, 8);
    let cmd = CreatureAttackStartLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 4,
        packet_already_broadcast: false,
    };

    assert_eq!(cmd.attacker_guid, attacker);
    assert_eq!(cmd.victim_guid, victim);
    assert_eq!(cmd.map_id, 571);
    assert_eq!(cmd.instance_id, 4);
}

#[tokio::test]
async fn durable_money_save_fence_closes_admission_then_waits_for_prior_worker() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let worker = tracker.begin_like_cpp().unwrap();
    let save_fence = tracker.close_admission_for_save_like_cpp();

    assert!(tracker.begin_like_cpp().is_err());
    let wait_tracker = Arc::clone(&tracker);
    let waiter = tokio::spawn(async move {
        wait_tracker.wait_until_idle_like_cpp().await;
    });
    tokio::task::yield_now().await;
    assert!(!waiter.is_finished());

    drop(worker);
    tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
        .await
        .expect("prior durable worker must release the save wait")
        .unwrap();
    assert!(tracker.begin_like_cpp().is_err());

    drop(save_fence);
    assert!(tracker.begin_like_cpp().is_ok());
}

#[test]
fn durable_money_permanent_logout_fence_never_reopens() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let save_fence = tracker.close_admission_for_save_like_cpp();
    tracker.close_admission_permanently_like_cpp();
    drop(save_fence);

    assert!(tracker.begin_like_cpp().is_err());
}

#[test]
fn overlapping_durable_money_save_fences_reopen_only_after_last_drop() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let first = tracker.close_admission_for_save_like_cpp();
    let second = tracker.close_admission_for_save_like_cpp();

    drop(first);
    assert!(
        tracker.begin_like_cpp().is_err(),
        "dropping the first fence must not reopen admission under the second"
    );
    drop(second);
    assert!(tracker.begin_like_cpp().is_ok());
}

#[test]
fn durable_money_completion_uses_one_shared_exact_once_gate() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let mut worker = tracker.begin_like_cpp().unwrap();
    let applied = Arc::new(AtomicBool::new(false));
    worker.commit_like_cpp(DurableLootMoneyCompletionLikeCpp {
        durable_money_before: 40,
        durable_money_after: 47,
        durable_applied_amount: 7,
        applied: Arc::clone(&applied),
    });

    let pending = tracker.pending_completions_like_cpp();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].durable_money_before, 40);
    assert_eq!(pending[0].durable_money_after, 47);
    assert_eq!(pending[0].durable_applied_amount, 7);
    assert!(
        pending[0]
            .applied
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_ok()
    );
    assert!(tracker.pending_completions_like_cpp().is_empty());
}

#[test]
fn indeterminate_money_outcome_permanently_closes_admission_until_relogin() {
    let tracker = Arc::new(DurableLootMoneyPersistenceTrackerLikeCpp::default());
    let fence = tracker.close_admission_for_save_like_cpp();
    tracker.mark_indeterminate_like_cpp();
    drop(fence);

    assert!(tracker.is_indeterminate_like_cpp());
    assert!(tracker.begin_like_cpp().is_err());
}

#[test]
fn durable_creature_runtime_commands_preserve_committed_fifo_like_cpp() {
    let victim = ObjectGuid::create_player(1, 900);
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        9001,
        901,
    );
    let mut pending = DurableCreatureRuntimeCommandsLikeCpp::default();
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
    assert!(
        pending.publish_pvp_combat_expiry_like_cpp(ReconcilePvpCombatExpiryLikeCppCommand {
            player_guid: victim,
            map_id: 571,
            instance_id: 0,
        })
    );
    for (victim_health_after, victim_health_state_revision_after) in [(90, 7), (75, 8)] {
        assert!(
            pending.publish_melee_damage_like_cpp(ApplyCreatureMeleeDamageLikeCppCommand {
                attacker_guid: attacker,
                victim_guid: victim,
                map_id: 571,
                instance_id: 0,
                damage: 15,
                over_damage: -1,
                target_level: 80,
                victim_health_after,
                victim_health_state_revision_after,
            })
        );
    }

    let commands = pending.drain_like_cpp();
    assert_eq!(commands.len(), 5);
    assert!(matches!(
        commands[0],
        SessionCommand::CreatureAttackStartLikeCpp(_)
    ));
    assert!(matches!(
        commands[1],
        SessionCommand::CreatureAttackStopLikeCpp(_)
    ));
    assert!(matches!(
        commands[2],
        SessionCommand::ReconcilePvpCombatExpiryLikeCpp(_)
    ));
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(first_melee) = &commands[3] else {
        panic!("expected first melee event");
    };
    let SessionCommand::ApplyCreatureMeleeDamageLikeCpp(second_melee) = &commands[4] else {
        panic!("expected second melee event");
    };
    assert_eq!(first_melee.victim_health_after, 90);
    assert_eq!(first_melee.victim_health_state_revision_after, 7);
    assert_eq!(second_melee.victim_health_after, 75);
    assert_eq!(second_melee.victim_health_state_revision_after, 8);
    assert!(pending.drain_like_cpp().is_empty());
}

#[test]
fn durable_creature_runtime_commands_bound_stalled_session_memory_like_cpp() {
    let victim = ObjectGuid::create_player(1, 902);
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        571,
        0,
        9001,
        903,
    );
    let command = CreatureAttackStartLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: victim,
        previous_victim_guid: None,
        map_id: 571,
        instance_id: 0,
        packet_already_broadcast: false,
    };
    let mut pending = DurableCreatureRuntimeCommandsLikeCpp::default();
    for _ in 0..MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
        assert!(pending.publish_attack_start_like_cpp(command.clone()));
    }
    assert!(!pending.publish_attack_start_like_cpp(command));
    assert!(pending.take_overflowed_and_discard_like_cpp());
    assert!(pending.drain_like_cpp().is_empty());
    assert!(!pending.take_overflowed_and_discard_like_cpp());
}
