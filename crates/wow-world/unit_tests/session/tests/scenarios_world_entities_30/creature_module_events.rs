// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `creature.added_to_world` and `creature.died` (#1444 H3b): the lifecycle
//! phase reports each transition exactly once, after it happened, and only
//! copies events when a module wants them.

use super::*;
use crate::session::{CreatureModuleEventKindLikeCpp, CreatureModuleEventLikeCpp};

fn kinds(
    events: &[CreatureModuleEventLikeCpp],
) -> Vec<(CreatureModuleEventKindLikeCpp, ObjectGuid)> {
    events
        .iter()
        .map(|event| (event.kind, event.guid))
        .collect()
}

fn creature_session(
    guid_counter: i64,
    health: u32,
) -> (WorldSession, SharedCanonicalMapManager, ObjectGuid) {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(guid_counter);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), guid, health);
    (session, manager, guid)
}

#[tokio::test]
async fn a_kill_reports_creature_died_once_on_the_next_lifecycle_phase() {
    let (mut session, canonical, guid) = creature_session(91_101, 40);
    let map_store = canonical_player_transfer_test_map_store_like_cpp();
    // Whatever the fixture's own add reported is consumed first.
    let _ = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    session.core.player_guid = Some(ObjectGuid::create_player(1, 9101));

    session.apply_damage(None, guid, 100).await.unwrap();
    let first = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    let second = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );

    assert_eq!(
        kinds(&first.module_events_like_cpp),
        vec![(CreatureModuleEventKindLikeCpp::Died, guid)]
    );
    let event = &first.module_events_like_cpp[0];
    let (entry, position) = session
        .mutate_world_creature(guid, |creature| (creature.entry(), creature.position()))
        .expect("the corpse is still on its map");
    assert_eq!((event.entry, event.position), (entry, position));
    assert!(
        second.module_events_like_cpp.is_empty(),
        "one JUST_DIED transition, one creature.died"
    );
}

#[tokio::test]
async fn without_a_creature_module_nothing_is_copied() {
    let (mut session, canonical, guid) = creature_session(91_102, 40);
    let map_store = canonical_player_transfer_test_map_store_like_cpp();
    session.core.player_guid = Some(ObjectGuid::create_player(1, 9102));
    session.apply_damage(None, guid, 100).await.unwrap();

    let silent = run_canonical_creature_lifecycle_phase_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );

    assert!(silent.module_events_like_cpp.is_empty());
}

#[tokio::test]
async fn a_forced_despawn_raises_no_creature_died() {
    // AzerothCore raises `OnUnitDeath` from `Unit::Kill` only
    // (`Unit.cpp:14307`), not from `ForcedDespawn`.
    let (mut session, canonical, guid) = creature_session(91_104, 40);
    let map_store = canonical_player_transfer_test_map_store_like_cpp();
    let _ = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    session
        .mutate_world_creature(guid, |creature| {
            let _ = creature.forced_despawn_runtime(0, 0, wow_entities::game_time_secs_like_cpp());
        })
        .expect("live creature");

    let outcome = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert!(
        outcome
            .module_events_like_cpp
            .iter()
            .all(|event| event.kind != CreatureModuleEventKindLikeCpp::Died),
        "{:?}",
        outcome.module_events_like_cpp
    );
}

#[test]
fn a_respawn_reports_creature_added_to_world() {
    let (mut session, _send_rx, guid) = setup_dead_creature_past_despawn(91_103);
    let canonical = session.core.canonical_map_manager.clone().unwrap();
    let map_store = canonical_player_transfer_test_map_store_like_cpp();
    let despawn = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert_eq!(despawn.corpses_despawned, 1);
    force_respawn_ready(&mut session);

    let respawn = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );
    assert_eq!(respawn.respawns_processed, 1);
    let next = run_canonical_creature_lifecycle_phase_recording_module_events_for_test_like_cpp(
        &canonical,
        &map_store,
        Instant::now(),
    );

    // The respawn is added to the map inside the phase, after its creatures
    // were visited, so it is reported by the next one.
    let added: Vec<_> = respawn
        .module_events_like_cpp
        .iter()
        .chain(&next.module_events_like_cpp)
        .filter(|event| event.guid == guid)
        .map(|event| event.kind)
        .collect();
    assert_eq!(added, vec![CreatureModuleEventKindLikeCpp::AddedToWorld]);
}
