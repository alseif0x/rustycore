//! #1263 C1 — controlled stale/ABA replay capture.
//!
//! This module is the **injected harness** half of capture C1. It drives the
//! real production mutation root
//! (`SessionCore::with_admitted_world_creature_like_cpp`, reached through
//! `SessionCore::mutate_world_creature`) with three representations of one
//! incarnation and records the structured `rustycore::capture::c1` events the
//! production instrumentation emits:
//!
//! 1. a **fresh, legitimate** representation, which the incarnation must admit
//!    and apply;
//! 2. a **stale** representation (the canonical incarnation advanced without
//!    it), which must be refused on the `revision` clause;
//! 3. an **ABA** replay (the canonical health tuple returned to its previous
//!    values while the revision advanced), which must be refused on the same
//!    clause instead of being mistaken for a fresh snapshot.
//!
//! Every replay is **injected**: the staleness is manufactured by this fixture,
//! not observed in a live run. The record built from this capture labels it
//! exactly that way. The live half of C1 is the melee mirror observed against a
//! running server and is not produced here.
//!
//! The capture is written to the path in `RUSTYCORE_C1_REPLAY_CAPTURE` when
//! that variable is set (the record's raw artifact); without it the module only
//! asserts in memory and writes nothing.

use super::r7a_canonical_mutation::*;
use super::*;
use std::io::Write as _;
use std::sync::{Arc, Mutex};

/// `MakeWriter` that appends every subscriber write to an in-memory buffer.
#[derive(Clone)]
struct CaptureSinkLikeCpp(Arc<Mutex<Vec<u8>>>);

struct CaptureSinkGuardLikeCpp<'a>(std::sync::MutexGuard<'a, Vec<u8>>);

impl std::io::Write for CaptureSinkGuardLikeCpp<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CaptureSinkLikeCpp {
    type Writer = CaptureSinkGuardLikeCpp<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        CaptureSinkGuardLikeCpp(
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

/// Drain everything the subscriber has written so far.
fn drain_capture_like_cpp(sink: &CaptureSinkLikeCpp) -> String {
    let mut buffer = sink
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let drained = buffer.clone();
    buffer.clear();
    String::from_utf8(drained).expect("the C1 capture sink holds UTF-8 subscriber output")
}

/// The single C1 event a drained capture chunk must contain.
fn one_event_like_cpp(chunk: &str, what: &str) -> String {
    let events: Vec<&str> = chunk
        .lines()
        .filter(|line| line.contains("RUSTYCORE_CAPTURE_C1"))
        .collect();
    assert_eq!(
        events.len(),
        1,
        "the {what} decision must emit exactly one C1 event, captured: {chunk}"
    );
    events[0].to_string()
}

#[test]
fn c1_injected_stale_and_aba_replay_capture_like_cpp() {
    let sink = CaptureSinkLikeCpp(Arc::new(Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_ansi(false)
        .with_target(true)
        // The harness records every decision the production instrumentation
        // emits, whatever level the running server would use for it.
        .with_max_level(tracing::Level::TRACE)
        .finish();

    // Distinct GUIDs keep the three replays independent.
    let fresh_guid = test_creature_guid(93_100);
    let stale_guid = test_creature_guid(93_101);
    let aba_guid = test_creature_guid(93_102);

    let (mut fresh_session, _fresh_manager, _fresh_canonical) =
        mirrored_session_like_cpp(fresh_guid, 100);
    let (mut stale_session, _stale_manager, stale_canonical) =
        mirrored_session_like_cpp(stale_guid, 100);
    let (mut aba_session, _aba_manager, aba_canonical) = mirrored_session_like_cpp(aba_guid, 100);

    // Manufacture the two rejection shapes before any capture is taken, so the
    // only C1 events in the sink belong to the replayed decisions.
    advance_canonical_max_health_like_cpp(&stale_canonical, stale_guid, 140);
    aba_canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical map instance")
        .map_mut()
        .with_creature_mut_like_cpp(aba_guid, |creature| {
            creature.unit_mut().set_health(60);
            creature.unit_mut().set_health(100);
        })
        .expect("canonical incarnation");

    let guard = tracing::subscriber::set_default(subscriber);
    let mut capture = String::new();

    let fresh_applied = fresh_session
        .core
        .mutate_world_creature(fresh_guid, |creature| {
            creature.creature.unit_mut().set_health(90);
        })
        .is_some();
    let fresh_chunk = drain_capture_like_cpp(&sink);
    capture.push_str(&fresh_chunk);

    let stale_invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stale_callback = stale_invoked.clone();
    let stale_result = stale_session
        .core
        .mutate_world_creature(stale_guid, move |creature| {
            stale_callback.store(true, std::sync::atomic::Ordering::SeqCst);
            creature.creature.unit_mut().set_health(50);
        });
    let stale_chunk = drain_capture_like_cpp(&sink);
    capture.push_str(&stale_chunk);

    let aba_invoked = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let aba_callback = aba_invoked.clone();
    let aba_result = aba_session
        .core
        .mutate_world_creature(aba_guid, move |creature| {
            aba_callback.store(true, std::sync::atomic::Ordering::SeqCst);
            creature.creature.unit_mut().set_health(50);
        });
    let aba_chunk = drain_capture_like_cpp(&sink);
    capture.push_str(&aba_chunk);

    drop(guard);

    assert!(
        fresh_applied,
        "the fresh representation is admitted and applied by the production root"
    );
    assert!(
        stale_result.is_none(),
        "a stale representation must be refused by the production root"
    );
    assert!(
        aba_result.is_none(),
        "an ABA replay must be refused by the production root"
    );
    assert!(
        !stale_invoked.load(std::sync::atomic::Ordering::SeqCst),
        "a refused mutation must not invoke the callback"
    );
    assert!(
        !aba_invoked.load(std::sync::atomic::Ordering::SeqCst),
        "a refused mutation must not invoke the callback"
    );

    let applied = one_event_like_cpp(&fresh_chunk, "fresh");
    assert!(
        applied.contains("root=\"SessionCore::with_admitted_world_creature_like_cpp\"")
            && applied.contains("admission_clause=\"admitted\"")
            && applied.contains("admission_verdict=true")
            && applied.contains("mutation_invoked=true")
            && applied.contains("application=\"applied\"")
            && applied.contains("representation_applied=true"),
        "the fresh decision reports an applied representation: {applied}"
    );
    assert!(
        applied.contains("shares_health_timeline=true")
            && applied.contains("health_tuple_matches=true"),
        "the fresh representation is one incarnation's current snapshot: {applied}"
    );

    // `stale` advances the canonical `max_health`, so its health tuple disagrees
    // with the transported one; `ABA` cycles health back to the previous values,
    // so its tuple *matches* and only the revision betrays the replay. That
    // difference is exactly what the clause must not confuse.
    for (what, chunk, tuple_matches) in [("stale", &stale_chunk, false), ("ABA", &aba_chunk, true)]
    {
        let refused = one_event_like_cpp(chunk, what);
        assert!(
            refused.contains("admission_clause=\"revision\"")
                && refused.contains("admission_verdict=false")
                && refused.contains("application=\"admission_refused\"")
                && refused.contains("representation_applied=false")
                && refused.contains("mutation_invoked=false"),
            "the {what} replay is refused on the revision clause: {refused}"
        );
        assert!(
            refused.contains("shares_health_timeline=true"),
            "the {what} replay is refused as one incarnation's stale snapshot: {refused}"
        );
        assert_eq!(
            refused.contains("health_tuple_matches=true"),
            tuple_matches,
            "the {what} replay reports the expected health-tuple agreement: {refused}"
        );
    }

    if let Some(path) = std::env::var_os("RUSTYCORE_C1_REPLAY_CAPTURE") {
        let path = std::path::PathBuf::from(path);
        let mut file = std::fs::File::create(&path)
            .unwrap_or_else(|error| panic!("create C1 replay capture {}: {error}", path.display()));
        file.write_all(capture.as_bytes())
            .unwrap_or_else(|error| panic!("write C1 replay capture {}: {error}", path.display()));
        file.flush().expect("flush C1 replay capture");
    }
}

// ---------------------------------------------------------------------------
// #1263 C1-R6 — a refused representation publishes no success (injected)
// ---------------------------------------------------------------------------
//
// Evidence label: **C1-R6-injected**. Both refusal shapes below are manufactured
// by this fixture — the canonical incarnation is advanced while the transported
// representation keeps its earlier state. Nothing in this section is a live
// observation, and the rejection has **no C++ counterpart**: the revision/ABA
// admission gate is a Rust safeguard. C++ `Unit`/`Creature` own one object in
// place, so there is no second snapshot that could be refused; the pairing in
// the R6 record is with that single-object in-place ownership contract, not with
// an ABA rejection the reference implements.

/// The two injected refusal shapes this section manufactures.
#[derive(Clone, Copy)]
enum R6RefusalShapeLikeCpp {
    /// The canonical incarnation advanced without its representation, so the
    /// transported health tuple disagrees with the incarnation's.
    Stale,
    /// Health left and returned to the same tuple while the revision advanced,
    /// so the tuple agrees and only the revision betrays the replay.
    Aba,
}

impl R6RefusalShapeLikeCpp {
    fn label_like_cpp(self) -> &'static str {
        match self {
            Self::Stale => "stale",
            Self::Aba => "ABA",
        }
    }
}

/// Manufacture one refusal shape on the canonical incarnation. `aba_restore_health`
/// is the tuple the ABA cycle returns to — the health the transported
/// representation carries — so the only difference left is the advanced revision.
fn inject_r6_refusal_shape_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    shape: R6RefusalShapeLikeCpp,
    aba_restore_health: u64,
) {
    match shape {
        R6RefusalShapeLikeCpp::Stale => {
            advance_canonical_max_health_like_cpp(canonical, guid, 140);
        }
        R6RefusalShapeLikeCpp::Aba => {
            canonical
                .lock()
                .unwrap()
                .find_map_mut(0, 0)
                .expect("canonical map instance")
                .map_mut()
                .with_creature_mut_like_cpp(guid, |creature| {
                    creature.unit_mut().set_health(60);
                    creature.unit_mut().set_health(aba_restore_health);
                })
                .expect("canonical incarnation");
        }
    }
}

/// Run one body under a subscriber that captures the production C1 events and
/// return the body's value together with everything the instrumentation emitted.
fn with_r6_capture_like_cpp<T>(body: impl FnOnce() -> T) -> (T, String) {
    let sink = CaptureSinkLikeCpp(Arc::new(Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_ansi(false)
        .with_target(true)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);
    let result = body();
    let chunk = drain_capture_like_cpp(&sink);
    drop(guard);
    (result, chunk)
}

fn r6_test_position_like_cpp() -> Position {
    Position::new(10.0, 10.0, 0.0, 0.0)
}

/// Queue one ready respawn for `guid` and run the production global lifecycle
/// tick once, so the legacy representation the movement tick carries exists.
fn r6_run_respawn_tick_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    hp: u32,
    now: Instant,
) -> crate::session::LegacyCreatureLifecycleTickOutcomeLikeCpp {
    use crate::map_manager::{RuntimeTickOwner, pending_respawn_from_world_creature_like_cpp};

    let queued = crate::map_manager::WorldCreature::new(
        guid,
        9001,
        r6_test_position_like_cpp(),
        hp,
        8,
        9,
        13,
        20.0,
        105,
        14,
        0,
        0,
    );
    let pending = pending_respawn_from_world_creature_like_cpp(&queued, now, 0);
    {
        let mut guard = manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
        guard.push_respawn(0, 0, pending);
    }
    run_legacy_creature_lifecycle_tick_once_like_cpp(
        manager,
        Some(canonical),
        &lifecycle_test_map_store_like_cpp(0, wow_data::map::MAP_COMMON, 0),
        now + Duration::from_secs(1),
    )
}

/// **C1-R6-injected.** Both the stale and the ABA refusal, driven through a
/// production *consumer* rather than by calling an admission root directly, and
/// covering **both** production admission roots:
///
/// * `SessionCore::with_admitted_world_creature_like_cpp`
///   (`crates/wow-world-core/src/session/world_entities/creature_registry.rs`),
///   reached through the loot-release owner consumer
///   (`LootReleaseOwnerAccessLikeCpp::finish_looted_creature_like_cpp`);
/// * `sync_admitted_creature_representation_on_map_like_cpp`
///   (`crates/wow-world-core/src/session/creature_canonical_adapter.rs`),
///   reached through the real creature movement tick.
///
/// For every case it asserts that the refusal happens **before** the callback
/// (`mutation_invoked=false` in the production instrumentation), that **both
/// stores** are unchanged, that the incarnation's **authority is preserved**, and
/// that the refusal produces **no successful client publication**.
///
/// The movement tick's own legitimate movement packet is not a publication of
/// the refused representation: that packet describes the representation's
/// movement, which is not what the gate refuses. What the gate must not do — and
/// what this asserts — is publish the refused snapshot *into* the incarnation, or
/// rebind the legacy alias onto an authority the incarnation refused.
#[test]
fn c1_r6_refusal_has_no_success_publication_like_cpp() {
    // ---- Root 1 (creature_registry.rs), through the loot-release consumer. ----
    for (index, shape) in [R6RefusalShapeLikeCpp::Stale, R6RefusalShapeLikeCpp::Aba]
        .into_iter()
        .enumerate()
    {
        let what = shape.label_like_cpp();
        let guid = test_creature_guid(94_000 + index as i64);
        let manager = shared_map_manager();
        let canonical = shared_canonical_map_manager();
        let (mut session, _pkt_tx, send_rx) = make_session();
        register_test_creature_mirrored_like_cpp(
            &mut session,
            manager.clone(),
            &canonical,
            guid,
            100,
        );

        let legacy_authority_before = legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .clone();
        let incarnation_authority_before = canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .loot_authority_like_cpp()
            .clone();

        inject_r6_refusal_shape_like_cpp(&canonical, guid, shape, 100);

        let legacy_before = observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
        );
        let canonical_before = observables_like_cpp(
            &canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation"),
        );

        let (released, chunk) = with_r6_capture_like_cpp(|| {
            session
                .core
                .loot_release_owner_access_like_cpp()
                .finish_looted_creature_like_cpp(guid, false, 0.5, None)
        });

        assert!(
            released.is_none(),
            "the {what} representation must be refused by the production loot-release consumer"
        );
        let event = one_event_like_cpp(&chunk, what);
        assert!(
            event.contains("root=\"SessionCore::with_admitted_world_creature_like_cpp\"")
                && event.contains("admission_clause=\"revision\"")
                && event.contains("admission_verdict=false")
                && event.contains("application=\"admission_refused\"")
                && event.contains("representation_applied=false"),
            "the {what} representation is refused on the revision clause: {event}"
        );
        assert!(
            event.contains("mutation_invoked=false"),
            "the refusal happens before the callback: {event}"
        );
        assert_eq!(
            observables_like_cpp(
                &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
            ),
            legacy_before,
            "the refused {what} release does not mutate the legacy representation"
        );
        assert_eq!(
            observables_like_cpp(
                &canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation")
            ),
            canonical_before,
            "the refused {what} release does not mutate the canonical incarnation"
        );
        assert!(
            canonical_creature_like_cpp(&canonical, guid)
                .expect("canonical incarnation")
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&incarnation_authority_before),
            "the {what} refusal preserves the incarnation's authority"
        );
        assert!(
            legacy_creature_like_cpp(&manager, guid)
                .expect("legacy representation")
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&legacy_authority_before),
            "the {what} refusal preserves the representation's authority"
        );
        assert!(
            drain_server_opcodes(&send_rx).is_empty(),
            "a refused {what} loot release publishes no successful client packet"
        );
    }

    // ---- Root 2 (creature_canonical_adapter.rs), through the movement tick. ----
    for (index, shape) in [R6RefusalShapeLikeCpp::Stale, R6RefusalShapeLikeCpp::Aba]
        .into_iter()
        .enumerate()
    {
        let what = shape.label_like_cpp();
        let guid = test_creature_guid(94_100 + index as i64);
        let manager = shared_map_manager();
        let canonical = shared_canonical_map_manager();
        canonical.lock().unwrap().create_world_map(0, 0);
        let now = Instant::now();

        // A canonical incarnation already exists while the legacy store is empty,
        // so the ready respawn publishes the representation the movement tick
        // later offers to the incarnation.
        add_canonical_test_creature_on_map(
            &canonical,
            guid,
            9001,
            r6_test_position_like_cpp(),
            0,
            0,
            0,
        );
        {
            let mut guard = canonical.lock().unwrap();
            let typed = guard
                .find_map_mut(0, 0)
                .unwrap()
                .map_mut()
                .get_typed_creature_mut(guid)
                .expect("the canonical incarnation pre-exists");
            typed.unit_mut().set_level(80);
            typed.unit_mut().set_max_health(105);
            typed.unit_mut().set_health(105);
        }
        let respawned = r6_run_respawn_tick_like_cpp(
            &manager,
            &canonical,
            guid,
            105,
            now - Duration::from_secs(1),
        );
        assert_eq!(
            respawned.respawns_processed, 1,
            "the ready respawn publishes the representation the movement tick carries"
        );

        let incarnation_authority_before = canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .loot_authority_like_cpp()
            .clone();
        let legacy_authority_before = legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .clone();

        inject_r6_refusal_shape_like_cpp(&canonical, guid, shape, 105);

        let canonical_before =
            canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
        let canonical_ai_before = canonical_before.ai_state();
        let canonical_observables_before = observables_like_cpp(&canonical_before);

        // Arm the production creature movement tick exactly as the runtime does.
        {
            use crate::map_manager::RuntimeTickOwner;

            let mut guard = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard.set_tick_owner(RuntimeTickOwner::GlobalLegacy);
            let creature = guard
                .find_creature_mut(0, 0, guid)
                .expect("legacy representation");
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x9130);
            creature.backdate_runtime_clock_for_test(Duration::from_millis(10));
        }

        let mmap_config = MMapRuntimeConfigLikeCpp {
            enabled: false,
            ..Default::default()
        };
        let (movement, chunk) = with_r6_capture_like_cpp(|| {
            run_legacy_creature_movement_tick_once_like_cpp(
                &manager,
                Some(&canonical),
                &mmap_config,
                None,
                &HashMap::new(),
                10,
            )
        });

        assert_eq!(
            movement.canonical_syncs, 1,
            "the queued {what} snapshot is an attempt and is counted even when refused"
        );
        let event = one_event_like_cpp(&chunk, what);
        assert!(
            event.contains("root=\"sync_admitted_creature_representation_on_map_like_cpp\"")
                && event.contains("admission_clause=\"revision\"")
                && event.contains("admission_verdict=false")
                && event.contains("representation_applied=false"),
            "the {what} mirror is refused on the revision clause: {event}"
        );
        assert!(
            event.contains("mutation_invoked=false"),
            "the mirror refusal happens before the application: {event}"
        );
        assert_eq!(
            canonical_creature_like_cpp(&canonical, guid)
                .expect("canonical incarnation")
                .ai_state(),
            canonical_ai_before,
            "the refused {what} mirror publishes no movement state to the incarnation"
        );
        assert_eq!(
            observables_like_cpp(
                &canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation")
            ),
            canonical_observables_before,
            "the refused {what} mirror leaves the incarnation untouched"
        );
        assert!(
            canonical_creature_like_cpp(&canonical, guid)
                .expect("canonical incarnation")
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&incarnation_authority_before),
            "the {what} mirror refusal preserves the incarnation's authority"
        );
        assert!(
            legacy_creature_like_cpp(&manager, guid)
                .expect("legacy representation")
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(&legacy_authority_before),
            "the {what} mirror refusal does not rebind the legacy alias onto a refused snapshot"
        );
    }
}
