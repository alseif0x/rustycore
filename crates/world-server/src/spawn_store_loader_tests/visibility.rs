//! Visibility scenarios for [`super`].
//!
//! Split out of spawn_store_loader_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_start_world_inactive_conditions_false_saves_without_nextphase_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event(
            1,
            GameEventStateLikeCpp::WorldInactive,
            0,
            0,
            0,
            7,
        )]));

    assert_eq!(
        metadata.start_game_event_like_cpp(1, true, 500, false),
        GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
            event_id: 1,
            state_before_raw: GameEventStateLikeCpp::WorldInactive as u8,
            state_after_raw: GameEventStateLikeCpp::WorldConditions as u8,
            active_added: true,
            active_was_present: false,
            apply_new_event_requested: true,
            save_world_event_state_requested: true,
            force_game_event_update_requested: false,
            completed: false,
        })
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(
        event.state_raw,
        GameEventStateLikeCpp::WorldConditions as u8
    );
    assert_eq!(event.next_start, 0);
    assert!(
        metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}
#[test]
fn game_event_start_serverwide_conditions_true_nextphase_and_force_flag_like_cpp() {
    for overwrite in [false, true] {
        let mut metadata =
            CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
                .with_game_events_like_cpp(game_event_store([event(
                    1,
                    GameEventStateLikeCpp::WorldConditions,
                    0,
                    0,
                    0,
                    7,
                )]));

        assert_eq!(
            metadata.start_game_event_like_cpp(1, overwrite, 500, true),
            GameEventStartOutcomeLikeCpp::Started(GameEventStartSummaryLikeCpp {
                event_id: 1,
                state_before_raw: GameEventStateLikeCpp::WorldConditions as u8,
                state_after_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
                active_added: true,
                active_was_present: false,
                apply_new_event_requested: true,
                save_world_event_state_requested: true,
                force_game_event_update_requested: overwrite,
                completed: true,
            })
        );
        let event = metadata.game_event_like_cpp(1).unwrap();
        assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldNextPhase as u8);
        assert_eq!(event.next_start, 920);
    }

    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store([event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7),
            777,
        )]));
    metadata.start_game_event_like_cpp(1, true, 500, true);
    assert_eq!(metadata.game_event_like_cpp(1).unwrap().next_start, 777);
}
#[test]
fn game_event_update_world_nextphase_finish_saves_stops_and_skips_nextcheck_like_cpp() {
    let mut metadata = CanonicalSpawnMetadataLikeCpp::new(SpawnStore::default(), BTreeMap::new())
        .with_game_events_like_cpp(game_event_store_with_max(
            2,
            [
                event_with_next_start(
                    event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 5),
                    500,
                ),
                event(2, GameEventStateLikeCpp::Normal, 100, 1_000, 10, 2),
            ],
        ));
    metadata
        .game_event_active_set_mut_like_cpp()
        .add_active_event_like_cpp(1);

    let outcome = metadata.update_game_events_like_cpp(500, true, |_| false);

    assert_eq!(
        outcome.world_nextphase_finished,
        vec![GameEventWorldNextPhaseFinishedLikeCpp {
            event_id: 1,
            was_active_before_queue: true,
            state_before_raw: GameEventStateLikeCpp::WorldNextPhase as u8,
            state_after_raw: GameEventStateLikeCpp::WorldFinished as u8,
            next_start_before: 500,
            next_start_after: 0,
            save_state_requested: true,
        }]
    );
    assert_eq!(outcome.queued_deactivation_event_ids, vec![1]);
    assert!(
        !outcome
            .next_check_outcomes
            .iter()
            .any(|(event_id, _)| *event_id == 1)
    );
    let event = metadata.game_event_like_cpp(1).unwrap();
    assert_eq!(event.state_raw, GameEventStateLikeCpp::WorldFinished as u8);
    assert_eq!(event.next_start, 0);
    assert!(
        !metadata
            .game_event_active_set_like_cpp()
            .is_active_event_like_cpp(1)
    );
}
#[test]
fn game_event_next_check_world_phase_and_conditions_like_cpp() {
    let store = game_event_store([
        event_with_next_start(
            event(1, GameEventStateLikeCpp::WorldNextPhase, 0, 0, 0, 0),
            700,
        ),
        event_with_next_start(
            event(2, GameEventStateLikeCpp::WorldFinished, 0, 0, 0, 0),
            650,
        ),
        event(3, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 7),
        event(4, GameEventStateLikeCpp::WorldConditions, 0, 0, 0, 0),
    ]);

    assert_eq!(
        store.next_check_like_cpp(1, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(100)
    );
    assert_eq!(
        store.next_check_like_cpp(2, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(50)
    );
    assert_eq!(
        store.next_check_like_cpp(3, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(420)
    );
    assert_eq!(
        store.next_check_like_cpp(4, 600),
        GameEventNextCheckOutcomeLikeCpp::DelaySecs(MAX_GAME_EVENT_CHECK_DELAY_SECS_LIKE_CPP)
    );
}
