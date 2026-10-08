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
