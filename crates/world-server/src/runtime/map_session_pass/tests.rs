//! #787 — the coordinator half of the split tick.
//!
//! These pin what the contract actually requires: a deadline may only end a
//! pass that never started, a pass that started is waited for however long it
//! takes, and a pass whose end cannot be observed leaves the tick unable to
//! continue. Order and identity are pinned beside them.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use wow_core::ObjectGuid;
use wow_world::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp, PlayerRegistry,
    PlayerSessionRegistrationLikeCpp,
};
use wow_world::session::mailbox::{
    DurableCreatureRuntimeCommandsLikeCpp, PlayerRecoveryReply, PlayerRecoveryStatus,
    RunMapPhasePassLikeCppCommand, RunMapPhasePassResultLikeCpp, RunPlayerRecoveryRequest,
    SessionPhasePassOutcomeLikeCpp, SessionPhasePermitStateLikeCpp, SessionPhaseRequestLikeCpp,
};

use super::super::map_tick::CanonicalMapSessionPassMapLikeCpp;
use super::{run_map_phase_session_passes_like_cpp, run_player_recovery_passes};

const COORDINATOR: u64 = 4242;

fn register(
    registry: &PlayerRegistry,
    guid: ObjectGuid,
) -> flume::Receiver<SessionPhaseRequestLikeCpp> {
    let (send_tx, _send_rx) = flume::bounded(8);
    let (command_tx, _command_rx) = flume::bounded(8);
    let (session_phase_tx, session_phase_rx) = flume::bounded(8);
    registry.register_or_replace(
        guid,
        PlayerSessionRegistrationLikeCpp {
            identity: PlayerDirectoryIdentityLikeCpp::new("MapPhase", 1, 0, 1, 1, 0, 2),
            placement: PlayerDirectoryPlacementLikeCpp {
                map_id: 1,
                instance_id: 0,
                position: Default::default(),
                is_in_world: true,
                level: 1,
                is_alive: true,
            },
            active_loot_rolls: vec![],
            realm_send_tx: send_tx.clone(),
            send_tx,
            command_tx,
            session_phase_tx,
            durable_creature_runtime_commands_like_cpp: Arc::new(Mutex::new(
                DurableCreatureRuntimeCommandsLikeCpp::default(),
            )),
            client_visible_guids_like_cpp: Default::default(),
            client_visible_transports_like_cpp: Default::default(),
            advanced_combat_logging_enabled_like_cpp: Default::default(),
            visibility_refresh_pending_like_cpp: Default::default(),
        },
        Default::default(),
    );
    session_phase_rx
}

/// Build the admitted-map plan the way the canonical tick does: the identities
/// come from a real manager, because a handle is only mintable there.
fn one_map(guids: Vec<ObjectGuid>) -> Vec<CanonicalMapSessionPassMapLikeCpp> {
    one_map_at(guids, wow_map::MapKey::new(1, 0))
}

fn one_map_at(
    guids: Vec<ObjectGuid>,
    key: wow_map::MapKey,
) -> Vec<CanonicalMapSessionPassMapLikeCpp> {
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(key.map_id, key.instance_id);
    let participants = guids
        .into_iter()
        .map(|guid| {
            let mut player = Box::new(wow_entities::Player::new(
                Some(u64::try_from(guid.counter()).unwrap()),
                false,
            ));
            player.unit_mut().world_mut().object_mut().create(guid);
            let handle = manager.install_detached_player_like_cpp(player).unwrap();
            manager
                .attach_player_like_cpp(handle, key, wow_core::Position::xyz(1.0, 2.0, 3.0))
                .unwrap();
            let (handle, residence_revision) =
                manager.current_player_admission_like_cpp(guid).unwrap();
            wow_map::MapSessionPassParticipantLikeCpp {
                guid,
                handle,
                residence_revision,
            }
        })
        .collect();
    vec![CanonicalMapSessionPassMapLikeCpp {
        key,
        incarnation: manager.map_incarnation_like_cpp(key).unwrap(),
        participants,
    }]
}

/// Wait for the request the coordinator is sending, without assuming how many
/// polls it needs to get there.
async fn await_request(
    rail: &flume::Receiver<SessionPhaseRequestLikeCpp>,
) -> RunMapPhasePassLikeCppCommand {
    let request = tokio::time::timeout(Duration::from_secs(5), rail.recv_async())
        .await
        .expect("the coordinator sends a map-phase request")
        .expect("the rail stays open");
    let SessionPhaseRequestLikeCpp::Map(command) = request else {
        panic!("expected a map-phase request");
    };
    command
}

async fn await_recovery_request(
    rail: &flume::Receiver<SessionPhaseRequestLikeCpp>,
) -> RunPlayerRecoveryRequest {
    let request = tokio::time::timeout(Duration::from_secs(5), rail.recv_async())
        .await
        .expect("the producer sends a recovery request")
        .expect("the rail stays open");
    let SessionPhaseRequestLikeCpp::PlayerRecovery(request) = request else {
        panic!("expected a PlayerRecovery request");
    };
    request
}

fn complete_recovery_request(request: RunPlayerRecoveryRequest, status: PlayerRecoveryStatus) {
    assert!(matches!(
        request.permit.claim_like_cpp(),
        wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
    ));
    let admission = request.admission;
    if status == PlayerRecoveryStatus::Unavailable {
        request.permit.complete_unavailable();
    } else {
        request.permit.complete_like_cpp();
    }
    request
        .response_tx
        .try_send(PlayerRecoveryReply {
            coordinator_id: admission.coordinator_id,
            tick_epoch: admission.tick_epoch,
            map_tick_epoch: admission.map_tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            status: Some(status),
            disconnecting: false,
        })
        .unwrap();
}

fn take_request(
    rail: &flume::Receiver<SessionPhaseRequestLikeCpp>,
) -> RunMapPhasePassLikeCppCommand {
    let request = rail.try_recv().expect("map-phase request queued");
    let SessionPhaseRequestLikeCpp::Map(command) = request else {
        panic!("expected a map-phase request");
    };
    command
}

/// Answer one queued map-phase request the way a session task would: claim the
/// permit before any effect, then report after the pass.
fn answer(rail: &flume::Receiver<SessionPhaseRequestLikeCpp>, tick_epoch: u64, dispatched: usize) {
    let command = take_request(rail);
    assert!(matches!(
        command.permit.claim_like_cpp(),
        wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
    ));
    command.permit.complete_like_cpp();
    command
        .response_tx
        .try_send(RunMapPhasePassResultLikeCpp {
            tick_epoch,
            outcome: SessionPhasePassOutcomeLikeCpp::Ran,
            dispatched,
            stopped_at_ineligible_head: false,
            disconnecting: false,
        })
        .expect("completion accepted");
}

#[tokio::test]
async fn a_completed_pass_is_counted_once_with_its_dispatched_packets() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 10);
    let rail = register(&registry, guid);
    let participants = one_map(vec![guid]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            7,
            50,
            Duration::from_millis(500),
        ),
        async {
            tokio::task::yield_now().await;
            answer(&rail, 7, 3);
        }
    );

    assert_eq!(pass.participants, 1);
    assert_eq!(pass.requested, 1);
    assert_eq!(pass.completed, 1);
    assert_eq!(pass.dispatched, 3);
    assert!(pass.quiescent_like_cpp());
}

#[tokio::test]
async fn a_wrong_tick_terminal_reply_abandons_this_tick_without_live_effects() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 11);
    let rail = register(&registry, guid);
    let participants = one_map(vec![guid]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            7,
            50,
            Duration::from_millis(500),
        ),
        async {
            tokio::task::yield_now().await;
            // A terminal pass with the wrong epoch proves it is quiescent,
            // while its untrusted lineage requires this tick to be abandoned.
            answer(&rail, 6, 9);
        }
    );

    assert_eq!(pass.completed, 1);
    assert_eq!(pass.dispatched, 0);
    assert!(pass.quiescent_like_cpp());
    assert!(pass.blocks_tick_tail);
}

#[tokio::test]
async fn a_pass_that_never_started_is_revoked_at_the_deadline_and_can_never_run() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 12);
    let rail = register(&registry, guid);
    let participants = one_map(vec![guid]);

    let pass = run_map_phase_session_passes_like_cpp(
        &participants,
        &registry,
        COORDINATOR,
        7,
        50,
        Duration::from_millis(20),
    )
    .await;

    assert_eq!(pass.requested, 1);
    assert_eq!(pass.completed, 0);
    assert_eq!(pass.revoked_before_start, 1);
    // Continuing is safe only because the request can no longer run: the
    // session reaching it now finds a revoked permit and produces no effect.
    assert!(pass.quiescent_like_cpp());
    let command = take_request(&rail);
    assert_eq!(
        command.permit.claim_like_cpp(),
        wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Revoked
    );
    assert_eq!(
        command.permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RevokedBeforeStart
    );
}

#[tokio::test]
async fn a_claimed_pass_is_waited_for_past_the_deadline_instead_of_being_abandoned() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 14);
    let rail = register(&registry, guid);
    let participants = one_map(vec![guid]);

    let (pass, mutated_after_deadline) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            7,
            50,
            Duration::from_millis(20),
        ),
        async {
            // A session that claims the pass and is still working when the
            // coordinator's deadline passes: this is the admitted operation
            // that the old "timeout means continue" behaviour would have run
            // concurrently with the rest of the tick.
            tokio::task::yield_now().await;
            let command = take_request(&rail);
            assert!(matches!(
                command.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            tokio::time::sleep(Duration::from_millis(80)).await;
            // The mutation happens here, well past the deadline. It is only
            // safe because the coordinator is still waiting.
            let mutated = true;
            command.permit.complete_like_cpp();
            let _ = command.response_tx.try_send(RunMapPhasePassResultLikeCpp {
                tick_epoch: 7,
                outcome: SessionPhasePassOutcomeLikeCpp::Ran,
                dispatched: 1,
                stopped_at_ineligible_head: false,
                disconnecting: false,
            });
            mutated
        }
    );

    assert!(mutated_after_deadline);
    assert_eq!(pass.completed, 1);
    assert_eq!(pass.revoked_before_start, 0);
    assert!(
        pass.stalled_ms > 0,
        "the stall must be reported, not hidden"
    );
    assert!(pass.quiescent_like_cpp());
}

#[tokio::test]
async fn a_claimed_pass_whose_session_disappears_blocks_the_tick() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 15);
    let rail = register(&registry, guid);
    let participants = one_map(vec![guid]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            7,
            50,
            Duration::from_millis(20),
        ),
        async {
            tokio::task::yield_now().await;
            let command = take_request(&rail);
            assert!(matches!(
                command.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            // The session task dies mid-pass: its mutations neither finished
            // nor rolled back, and the response channel closes.
            drop(command);
        }
    );

    assert_eq!(pass.completed, 0);
    assert_eq!(pass.unresolved_after_start, 1);
    assert!(
        !pass.quiescent_like_cpp(),
        "the tick must not resume over an unfinished admitted pass"
    );
}

#[tokio::test]
async fn a_player_with_no_current_address_is_not_a_failed_pass() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 13);
    let participants = one_map(vec![guid]);

    let pass = run_map_phase_session_passes_like_cpp(
        &participants,
        &registry,
        COORDINATOR,
        7,
        50,
        Duration::from_millis(20),
    )
    .await;

    assert_eq!(pass.participants, 1);
    assert_eq!(pass.unaddressable, 1);
    assert_eq!(pass.requested, 0);
    assert!(pass.quiescent_like_cpp());
}

#[tokio::test]
async fn one_map_s_sessions_run_serially_in_the_plan_s_order() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 21);
    let second = ObjectGuid::create_player(1, 22);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    // The plan carries `m_mapRefManager` order, not sorted GUIDs.
    let participants = one_map(vec![second, first]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            9,
            50,
            Duration::from_millis(500),
        ),
        async {
            tokio::task::yield_now().await;
            // The session listed second has not been asked yet: the first in
            // the plan is driven and awaited before the next one starts.
            assert!(first_rail.is_empty());
            answer(&second_rail, 9, 1);
            tokio::task::yield_now().await;
            answer(&first_rail, 9, 1);
        }
    );

    assert_eq!(pass.completed, 2);
    assert_eq!(pass.dispatched, 2);
    assert!(pass.quiescent_like_cpp());
}

#[tokio::test]
async fn a_pending_db_result_in_one_session_holds_the_next_session_of_the_same_map() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 31);
    let second = ObjectGuid::create_player(1, 32);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            11,
            50,
            Duration::from_millis(20),
        ),
        async {
            tokio::task::yield_now().await;
            // The first session claims its pass and is waiting on persistence
            // when the coordinator's deadline expires.
            let command = await_request(&first_rail).await;
            assert!(matches!(
                command.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            tokio::time::sleep(Duration::from_millis(60)).await;
            // C++ never has two sessions of one map running together, because
            // the map thread runs them one after another inside its own update.
            assert!(
                second_rail.is_empty(),
                "the next session of this map must not be asked while the first is still running"
            );
            command.permit.complete_like_cpp();
            let _ = command.response_tx.try_send(RunMapPhasePassResultLikeCpp {
                tick_epoch: 11,
                outcome: SessionPhasePassOutcomeLikeCpp::Ran,
                dispatched: 1,
                stopped_at_ineligible_head: false,
                disconnecting: false,
            });
            let second = await_request(&second_rail).await;
            assert!(matches!(
                second.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            second.permit.complete_like_cpp();
            let _ = second.response_tx.try_send(RunMapPhasePassResultLikeCpp {
                tick_epoch: 11,
                outcome: SessionPhasePassOutcomeLikeCpp::Ran,
                dispatched: 1,
                stopped_at_ineligible_head: false,
                disconnecting: false,
            });
        }
    );

    assert_eq!(pass.completed, 2);
    assert!(pass.stalled_ms > 0);
    assert!(pass.quiescent_like_cpp());
}

#[tokio::test]
async fn an_unresolved_pass_stops_the_tick_and_hands_its_permit_to_the_producer() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 41);
    let second = ObjectGuid::create_player(1, 42);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (pass, ()) = tokio::join!(
        run_map_phase_session_passes_like_cpp(
            &participants,
            &registry,
            COORDINATOR,
            13,
            50,
            Duration::from_millis(20),
        ),
        async {
            let command = await_request(&first_rail).await;
            assert!(matches!(
                command.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            // The session dies mid-pass: its mutations neither finished nor
            // rolled back, and nothing will ever report their end.
            drop(command);
        }
    );

    assert_eq!(pass.unresolved_after_start, 1);
    assert!(!pass.quiescent_like_cpp());
    // The next player of this map is never asked: C++ would still be inside the
    // first session's pass.
    assert!(
        second_rail.is_empty(),
        "the pass must stop at the participant whose effects are unknown"
    );
    // The producer receives the permit, so its barrier can outlive this tick
    // and release only when that pass finally resolves.
    assert_eq!(pass.unresolved_permits.len(), 1);
    assert_eq!(
        pass.unresolved_permits[0].state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Running
    );
}

#[tokio::test]
async fn player_recovery_passes_are_serial_and_carry_both_epoch_lineages() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 51);
    let second = ObjectGuid::create_player(1, 52);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (summary, ()) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            900,
            41,
            7,
            37,
            Duration::from_millis(20),
        ),
        async {
            let first_request = await_recovery_request(&first_rail).await;
            assert_eq!(first_request.admission.coordinator_id, 900);
            assert_eq!(first_request.admission.tick_epoch, 41);
            assert_eq!(first_request.admission.map_tick_epoch, 7);
            assert_eq!(first_request.admission.diff_ms, 37);
            complete_recovery_request(first_request, PlayerRecoveryStatus::Completed);

            let second_request = await_recovery_request(&second_rail).await;
            complete_recovery_request(second_request, PlayerRecoveryStatus::PlayerLeft);
        }
    );

    assert_eq!(summary.participants, 2);
    assert_eq!(summary.requested, 2);
    assert_eq!(summary.completed, 2);
    assert!(summary.quiescent_like_cpp());
    assert!(!summary.blocks_tick_tail);
}

#[tokio::test]
async fn player_recovery_passes_are_serial_across_maps() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 53);
    let second = ObjectGuid::create_player(1, 54);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = [
        one_map_at(vec![first], wow_map::MapKey::new(1, 0))[0].clone(),
        one_map_at(vec![second], wow_map::MapKey::new(2, 0))[0].clone(),
    ];

    let (summary, ()) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            906,
            47,
            13,
            46,
            Duration::from_millis(20),
        ),
        async {
            let first_request = await_recovery_request(&first_rail).await;
            assert_eq!(first_request.admission.map_key.map_id, 1);
            assert!(
                second_rail.is_empty(),
                "later maps wait for this map's pass"
            );
            complete_recovery_request(first_request, PlayerRecoveryStatus::Completed);
            let second_request = await_recovery_request(&second_rail).await;
            assert_eq!(second_request.admission.map_key.map_id, 2);
            complete_recovery_request(second_request, PlayerRecoveryStatus::Completed);
        }
    );

    assert_eq!(summary.participants, 2);
    assert_eq!(summary.requested, 2);
    assert_eq!(summary.completed, 2);
    assert!(summary.quiescent_like_cpp());
}

#[tokio::test]
async fn player_recovery_unavailable_is_terminal_but_blocks_and_stops_this_tick() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 61);
    let second = ObjectGuid::create_player(1, 62);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (summary, ()) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            901,
            42,
            8,
            39,
            Duration::from_millis(20),
        ),
        async {
            let request = await_recovery_request(&first_rail).await;
            complete_recovery_request(request, PlayerRecoveryStatus::Unavailable);
            tokio::time::sleep(Duration::from_millis(5)).await;
            assert!(second_rail.is_empty(), "Unavailable stops later Players");
        }
    );

    assert!(summary.quiescent_like_cpp());
    assert_eq!(summary.completed, 1);
    assert!(summary.blocks_tick_tail);
    assert!(summary.unresolved_permits.is_empty());
}

#[tokio::test]
async fn player_recovery_unavailable_without_reply_still_blocks_the_tail() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 63);
    let second = ObjectGuid::create_player(1, 64);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (summary, (permit, _response_tx)) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            904,
            45,
            11,
            44,
            Duration::from_millis(20),
        ),
        async {
            let request = await_recovery_request(&first_rail).await;
            assert!(matches!(
                request.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            let permit = Arc::clone(&request.permit);
            let response_tx = request.response_tx.clone();
            assert!(request.permit.complete_unavailable());
            // Leave the channel open without a reply. The coordinator's next
            // timeout must recover the tail fence from the primary state.
            drop(request);
            (permit, response_tx)
        }
    );

    assert!(summary.quiescent_like_cpp());
    assert!(summary.blocks_tick_tail);
    assert_eq!(summary.completed, 1);
    assert!(summary.unresolved_permits.is_empty());
    assert!(second_rail.is_empty(), "Unavailable stops later Players");
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::CompletedUnavailable
    );
}

#[tokio::test]
async fn a_wrong_lineage_reply_revokes_a_recovery_request_that_has_not_started() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 65);
    let second = ObjectGuid::create_player(1, 66);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (summary, permit) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            905,
            46,
            12,
            45,
            Duration::from_millis(20),
        ),
        async {
            let request = await_recovery_request(&first_rail).await;
            let permit = Arc::clone(&request.permit);
            let admission = request.admission;
            request
                .response_tx
                .try_send(PlayerRecoveryReply {
                    coordinator_id: admission.coordinator_id,
                    tick_epoch: admission.tick_epoch,
                    map_tick_epoch: admission.map_tick_epoch + 1,
                    outcome: SessionPhasePassOutcomeLikeCpp::Ran,
                    status: Some(PlayerRecoveryStatus::Completed),
                    disconnecting: false,
                })
                .unwrap();
            permit
        }
    );

    assert!(summary.quiescent_like_cpp());
    assert!(summary.blocks_tick_tail);
    assert_eq!(summary.revoked_before_start, 1);
    assert_eq!(summary.completed, 0);
    assert!(summary.unresolved_permits.is_empty());
    assert!(
        second_rail.is_empty(),
        "untrusted lineage stops later Players"
    );
    assert_eq!(
        permit.claim_like_cpp(),
        wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Revoked
    );
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::RevokedBeforeStart
    );
}

#[tokio::test]
async fn player_recovery_does_not_treat_a_running_permit_reply_as_completion() {
    let registry = PlayerRegistry::new();
    let first = ObjectGuid::create_player(1, 71);
    let second = ObjectGuid::create_player(1, 72);
    let first_rail = register(&registry, first);
    let second_rail = register(&registry, second);
    let participants = one_map(vec![first, second]);

    let (summary, permit) = tokio::join!(
        run_player_recovery_passes(
            &participants,
            &registry,
            902,
            43,
            9,
            41,
            Duration::from_millis(20),
        ),
        async {
            let request = await_recovery_request(&first_rail).await;
            assert!(matches!(
                request.permit.claim_like_cpp(),
                wow_world::session::mailbox::SessionPhaseClaimLikeCpp::Claimed
            ));
            let permit = Arc::clone(&request.permit);
            let admission = request.admission;
            request
                .response_tx
                .try_send(PlayerRecoveryReply {
                    coordinator_id: admission.coordinator_id,
                    tick_epoch: admission.tick_epoch,
                    map_tick_epoch: admission.map_tick_epoch,
                    outcome: SessionPhasePassOutcomeLikeCpp::Ran,
                    status: Some(PlayerRecoveryStatus::Completed),
                    disconnecting: false,
                })
                .unwrap();
            permit
        }
    );

    assert!(!summary.quiescent_like_cpp());
    assert_eq!(summary.unresolved_after_start, 1);
    assert_eq!(summary.unresolved_permits.len(), 1);
    assert_eq!(
        summary.unresolved_permits[0].state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Running
    );
    assert!(
        second_rail.is_empty(),
        "a receipt-like reply cannot advance Actor 2"
    );
    assert!(Arc::ptr_eq(&summary.unresolved_permits[0], &permit));
    assert!(permit.complete_like_cpp());
    assert_eq!(
        permit.state_like_cpp(),
        SessionPhasePermitStateLikeCpp::Completed
    );
}

#[tokio::test]
async fn player_recovery_refuses_a_full_bounded_phase_rail_without_effects() {
    let registry = PlayerRegistry::new();
    let guid = ObjectGuid::create_player(1, 81);
    let rail = register(&registry, guid);
    let address = registry.session_phase_address_like_cpp(guid).unwrap();
    for epoch in 0..8 {
        address
            .try_send(SessionPhaseRequestLikeCpp::World(
                wow_world::session::mailbox::RunWorldPhasePassLikeCppRequest {
                    coordinator_id: 1,
                    tick_epoch: epoch,
                    diff_ms: 1,
                    permit: wow_world::session::mailbox::SessionPhasePermitLikeCpp::new_like_cpp(),
                    response_tx: flume::bounded(1).0,
                },
            ))
            .unwrap();
    }
    let participants = one_map(vec![guid]);

    let summary = run_player_recovery_passes(
        &participants,
        &registry,
        903,
        44,
        10,
        43,
        Duration::from_millis(20),
    )
    .await;

    assert_eq!(summary.participants, 1);
    assert_eq!(summary.requested, 0);
    assert_eq!(summary.send_failed, 1);
    assert!(summary.quiescent_like_cpp());
    assert_eq!(rail.len(), 8);
}
