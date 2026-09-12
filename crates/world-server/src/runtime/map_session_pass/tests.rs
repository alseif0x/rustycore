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
    DurableCreatureRuntimeCommandsLikeCpp, RunMapPhasePassLikeCppCommand,
    RunMapPhasePassResultLikeCpp, SessionPhasePassOutcomeLikeCpp, SessionPhasePermitStateLikeCpp,
    SessionPhaseRequestLikeCpp,
};

use super::super::map_tick::CanonicalMapSessionPassMapLikeCpp;
use super::run_map_phase_session_passes_like_cpp;

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
    let key = wow_map::MapKey::new(1, 0);
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
async fn a_completion_for_another_tick_leaves_this_tick_unable_to_continue() {
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
            // A report carrying the previous tick's epoch, produced by a pass
            // that did claim and mutate.
            answer(&rail, 6, 9);
        }
    );

    assert_eq!(pass.completed, 0);
    assert_eq!(pass.dispatched, 0);
    // The request this tick sent was claimed by someone; nothing here proves
    // its effects ended, so the tick may not resume.
    assert!(!pass.quiescent_like_cpp());
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
