//! #1263 C1 — the creature mutation root's capture.
//!
//! The C1 capture records what the one creature mutation root
//! (`SessionCore::with_admitted_world_creature_like_cpp`, reached through
//! `SessionCore::mutate_world_creature` and the guarded loot mutators) did with
//! one mutation: which root ran, on which creature and map key, whether the
//! mutation callback ran and what became of it.
//!
//! Until #1263 F6-8D3b-2 the root admitted a transported legacy representation
//! against the canonical incarnation and the capture recorded that admission
//! decision (clause, revisions, timelines, allocations). F6-8D3b-2 made the
//! root canonical-first, which made the decision tautological (the incarnation
//! against itself, always admitted), and F6-8E deleted the legacy copy, the
//! decision and the legacy → canonical synchronization root whose refusals
//! (the R6 shape) the capture used to replay. What the capture still means is
//! the root outcome, pinned here for every outcome the root can report.

use super::r7a_canonical_mutation::*;
use super::*;
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
        "the {what} mutation must emit exactly one C1 event, captured: {chunk}"
    );
    events[0].to_string()
}

fn assert_root_outcome_like_cpp(
    event: &str,
    what: &str,
    mutation_invoked: bool,
    application: &str,
    applied: bool,
) {
    assert!(
        event.contains("root=\"SessionCore::with_admitted_world_creature_like_cpp\""),
        "{what}: the event names the mutation root: {event}"
    );
    assert!(
        event.contains(&format!("mutation_invoked={mutation_invoked}")),
        "{what}: mutation_invoked={mutation_invoked}: {event}"
    );
    assert!(
        event.contains(&format!("application=\"{application}\"")),
        "{what}: application={application}: {event}"
    );
    assert!(
        event.contains(&format!(" applied={applied}")),
        "{what}: applied={applied}: {event}"
    );
    for retired in [
        "admission_clause",
        "admission_verdict",
        "representation_applied",
        "representation_health_revision",
    ] {
        assert!(
            !event.contains(retired),
            "{what}: the retired admission field `{retired}` must not be recorded: {event}"
        );
    }
}

#[test]
fn c1_capture_records_every_mutation_root_outcome_like_cpp() {
    let sink = CaptureSinkLikeCpp(Arc::new(Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(sink.clone())
        .with_ansi(false)
        .with_target(true)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    let guid = test_creature_guid(93_100);
    let absent_guid = test_creature_guid(93_101);
    let (mut session, _manager, _canonical) = mirrored_session_like_cpp(guid, 100);
    let foreign = wow_loot::OwnedLootAuthority::new();
    let mut runs = 0_usize;

    tracing::subscriber::with_default(subscriber, || {
        // Applied: the callback runs once on the incarnation.
        assert!(
            session
                .mutate_world_creature(guid, |creature| {
                    runs += 1;
                    creature.ai_ownership_mut().npc_flags = 0x0000_0042;
                })
                .is_some()
        );
        let applied = one_event_like_cpp(&drain_capture_like_cpp(&sink), "applied");
        assert_root_outcome_like_cpp(&applied, "applied", true, "applied", true);
        assert!(applied.contains(&format!("owner_guid={guid}")));
        assert!(applied.contains("map_id=0") && applied.contains("instance_id=0"));

        // Refused by the mutation: a guarded loot mutator whose observed
        // authority is not the incarnation's own allocation.
        assert!(
            session
                .core
                .mutate_world_creature_if_fully_looted_observation_like_cpp(
                    guid,
                    &foreign,
                    0,
                    0,
                    |_| {
                        runs += 1;
                    },
                )
                .is_none()
        );
        let refused = one_event_like_cpp(&drain_capture_like_cpp(&sink), "refused");
        assert_root_outcome_like_cpp(&refused, "refused", true, "mutation_root_refused", false);

        // No incarnation for the GUID on the addressed map.
        assert!(
            session
                .mutate_world_creature(absent_guid, |_| {
                    runs += 1;
                })
                .is_none()
        );
        let absent = one_event_like_cpp(&drain_capture_like_cpp(&sink), "absent");
        assert_root_outcome_like_cpp(&absent, "absent", false, "incarnation_absent", false);

        // No canonical map for the session's key.
        session.set_canonical_map_manager(shared_map_manager());
        assert!(
            session
                .mutate_world_creature(guid, |_| {
                    runs += 1;
                })
                .is_none()
        );
        let unavailable = one_event_like_cpp(&drain_capture_like_cpp(&sink), "unavailable");
        assert_root_outcome_like_cpp(
            &unavailable,
            "unavailable",
            false,
            "owner_unavailable",
            false,
        );
    });

    assert_eq!(runs, 1, "only the applied mutation ran its callback");
}
