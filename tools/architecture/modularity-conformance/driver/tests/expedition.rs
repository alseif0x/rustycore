//! Post-freeze independent module acceptance through the REAL three-module factory.
//! No frozen oracle/core change; exact pairwise equality includes callback returns.

use conformance_contract::{Fault, Handle, Result, State, event};
use conformance_driver::composition::{self, Harness, Mode};
use conformance_expedition::{COUNT, ExpeditionState, MODULE_ID, STAMP, STATE_LIMIT};
use conformance_host::{EntitySnapshot, Executor, Limits, Trace};
use std::fmt::Debug;

struct Pair {
    native: Harness,
    actual: Harness,
    handles: Vec<Handle>,
    executed: usize,
}

fn semantic_snapshot(mut snapshot: EntitySnapshot) -> EntitySnapshot {
    // Only execution provenance differs. Retain every state byte, schema, ABI,
    // identity, revision, contribution and ordered module membership exactly.
    for module in &mut snapshot.modules {
        module.executor = Executor::Native;
    }
    snapshot
}

impl Pair {
    fn new(mode: Mode, limits: Limits) -> Self {
        let native = composition::build_with_limits(Mode::Native, limits).unwrap();
        let actual = composition::build_with_limits(mode, limits).unwrap();
        let ids: Vec<_> = actual.registered().iter().map(|m| m.id).collect();
        assert_eq!(ids, [1, 2, MODULE_ID]);
        assert_eq!(
            actual.executor(MODULE_ID),
            Ok(if mode == Mode::Native {
                Executor::Native
            } else {
                Executor::Wasm
            })
        );
        if mode == Mode::Mixed {
            assert_eq!(actual.executor(1), Ok(Executor::Native));
            assert_eq!(actual.executor(2), Ok(Executor::Wasm));
        }
        Self {
            native,
            actual,
            handles: Vec::new(),
            executed: 0,
        }
    }

    fn compare(&self, label: &str) {
        assert_eq!(
            self.actual.registered(),
            self.native.registered(),
            "{label}: manifests"
        );
        assert_eq!(
            self.actual.trace(),
            self.native.trace(),
            "{label}: full ordered trace"
        );
        assert_eq!(self.actual.calls(), self.native.calls(), "{label}: calls");
        assert_eq!(self.actual.depth(), 0, "{label}: leaked actual frame");
        assert_eq!(self.native.depth(), 0, "{label}: leaked native frame");
        for handle in &self.handles {
            assert_eq!(
                self.actual.observables(*handle),
                self.native.observables(*handle),
                "{label}: observables {handle:?}"
            );
            assert_eq!(
                self.actual.snapshot(*handle).map(semantic_snapshot),
                self.native.snapshot(*handle).map(semantic_snapshot),
                "{label}: complete snapshot {handle:?}"
            );
        }
    }

    fn step<T: Debug + Eq>(
        &mut self,
        label: &str,
        mut operation: impl FnMut(&mut Harness) -> Result<T>,
    ) -> Result<T> {
        let expected = operation(&mut self.native);
        let actual = operation(&mut self.actual);
        assert_eq!(actual, expected, "{label}: root result");
        self.executed += self
            .actual
            .trace()
            .iter()
            .filter(|entry| matches!(entry, Trace::Enter(frame) if frame.module == MODULE_ID))
            .count();
        self.compare(label);
        actual
    }

    fn state(&self, handle: Handle) -> ExpeditionState {
        ExpeditionState::decode(&self.actual.state(handle, MODULE_ID).unwrap().bytes).unwrap()
    }

    fn assert_unchanged(&self, handle: Handle, before: &EntitySnapshot) {
        assert_eq!(&self.actual.snapshot(handle).unwrap(), before);
    }
}

fn codec_rejections(pair: &mut Pair, handle: Handle) {
    for (case, fault) in [
        ("magic", Fault::Invalid),
        ("encoding_version", Fault::Invalid),
        ("unsorted", Fault::Invalid),
        ("duplicate", Fault::Invalid),
        ("invalid_id", Fault::Invalid),
        ("count", Fault::Invalid),
        ("history", Fault::Invalid),
        ("truncated", Fault::Invalid),
        ("trailing", Fault::Invalid),
        ("oversize", Fault::Limit),
        ("schema", Fault::Version),
    ] {
        let before = pair.actual.snapshot(handle).unwrap();
        assert_eq!(
            pair.step(case, |host| {
                let mut snapshot = host.snapshot(handle)?;
                let record = snapshot
                    .modules
                    .iter_mut()
                    .find(|m| m.id == MODULE_ID)
                    .unwrap();
                match case {
                    "magic" => record.bytes[0] = 0,
                    "encoding_version" => record.bytes[1] = 2,
                    "unsorted" => record.bytes.swap(15, 17),
                    "duplicate" => record.bytes[16] = record.bytes[15],
                    "invalid_id" => record.bytes[17] = 32,
                    "count" => record.bytes[14] = 2,
                    "history" => record.bytes[6..14].copy_from_slice(&2_u64.to_le_bytes()),
                    "truncated" => {
                        record.bytes.pop();
                    }
                    "trailing" => record.bytes.push(10),
                    "oversize" => record.bytes.resize(STATE_LIMIT + 1, 0),
                    "schema" => record.schema += 1,
                    _ => unreachable!(),
                }
                host.replay(handle, &snapshot)
            }),
            Err(fault),
            "{case}"
        );
        pair.assert_unchanged(handle, &before);
    }
}

fn lifecycle(mode: Mode) {
    let mut pair = Pair::new(mode, Limits::default());
    let mut handle = pair.step("spawn", |h| h.spawn(7300, 0)).unwrap();
    pair.handles.push(handle);
    let other = pair
        .step("independent entity", |h| h.spawn(7301, 0))
        .unwrap();
    pair.handles.push(other);
    pair.compare("default states");
    assert_eq!(
        pair.actual.entity_modules(handle).unwrap(),
        [1, 2, MODULE_ID]
    );
    assert_eq!(
        pair.actual.state(handle, MODULE_ID).unwrap().bytes.len(),
        15
    );

    assert_eq!(
        pair.step("real ordered policy fanout", |h| h.dispatch(
            handle,
            event::POLICY,
            80
        )),
        Ok(vec![(1, 0), (2, 80), (MODULE_ID, 0)])
    );
    for (checkpoint, count) in [(7, 1), (2, 2), (9, 3)] {
        assert_eq!(
            pair.step("stamp", |h| h
                .dispatch_one(handle, MODULE_ID, STAMP, checkpoint)),
            Ok(count)
        );
    }
    assert_eq!(
        pair.state(handle),
        ExpeditionState {
            resets: 0,
            accepted_total: 3,
            checkpoints: vec![2, 7, 9],
        }
    );
    assert_eq!(pair.state(other), ExpeditionState::default());
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 115);
    let before = pair.actual.snapshot(handle).unwrap();
    assert_eq!(
        pair.step("idempotent duplicate", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 7)),
        Ok(3)
    );
    pair.assert_unchanged(handle, &before);
    for argument in [-1, 0, 32, i64::MAX] {
        assert_eq!(
            pair.step("invalid checkpoint", |h| h
                .dispatch_one(handle, MODULE_ID, STAMP, argument)),
            Err(Fault::Invalid)
        );
        pair.assert_unchanged(handle, &before);
    }
    assert_eq!(
        pair.step("reject fabricated attach", |h| h.dispatch_one(
            handle,
            MODULE_ID,
            event::ATTACHED,
            1
        )),
        Err(Fault::Invalid)
    );
    assert_eq!(
        pair.step("reject fabricated detach", |h| h.dispatch_one(
            handle,
            MODULE_ID,
            event::DETACHED,
            0
        )),
        Err(Fault::Invalid)
    );
    pair.assert_unchanged(handle, &before);

    assert_eq!(
        pair.step("encounter reentry includes third", |h| h.dispatch_one(
            handle,
            1,
            event::UPDATE,
            1
        )),
        Ok(1)
    );
    let trace = pair.actual.trace();
    let callback_leave = trace
        .iter()
        .position(|entry| {
            matches!(entry, Trace::Leave { invocation, result: Ok(0) }
            if invocation.module == MODULE_ID && invocation.event == event::CALLBACK)
        })
        .expect("third module must complete the actual synchronous callback");
    let outer_leave = trace
        .iter()
        .position(|entry| {
            matches!(entry, Trace::Leave { invocation, .. }
            if invocation.module == 1 && invocation.event == event::UPDATE)
        })
        .unwrap();
    assert!(callback_leave < outer_leave);
    assert_eq!(pair.state(handle).accepted_total, 3);
    assert!(pair.actual.observables(handle).unwrap().shield);

    codec_rejections(&mut pair, handle);
    let before = pair.actual.state(handle, MODULE_ID).unwrap();
    pair.step("canonical same-incarnation replay", |h| {
        let snapshot = h.snapshot(handle)?;
        h.replay(handle, &snapshot)
    })
    .unwrap();
    let after = pair.actual.state(handle, MODULE_ID).unwrap();
    assert_eq!(after.bytes, before.bytes);
    assert!(after.revision > before.revision);
    assert_eq!(
        pair.step("old snapshot cannot undo new stamp", |h| {
            let old = h.snapshot(handle)?;
            h.dispatch_one(handle, MODULE_ID, STAMP, 4)?;
            h.replay(handle, &old)
        }),
        Err(Fault::Revision)
    );
    assert_eq!(pair.state(handle).accepted_total, 4);

    for checkpoint in [1, 3, 5, 6] {
        pair.step("fill variable state", |h| {
            h.dispatch_one(handle, MODULE_ID, STAMP, checkpoint)
        })
        .unwrap();
    }
    assert_eq!(
        pair.actual.state(handle, MODULE_ID).unwrap().bytes.len(),
        STATE_LIMIT
    );
    let full = pair.actual.snapshot(handle).unwrap();
    assert_eq!(
        pair.step("full set rejects new checkpoint", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 31)),
        Err(Fault::Limit)
    );
    assert_eq!(
        pair.step("full set still accepts duplicate", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 2)),
        Ok(8)
    );
    pair.assert_unchanged(handle, &full);

    let retained = pair.actual.state(handle, MODULE_ID).unwrap();
    let address = pair.actual.payload_identity(handle).unwrap();
    pair.step("detach suspends own effect", |h| h.detach(handle))
        .unwrap();
    assert_eq!(pair.actual.state(handle, MODULE_ID).unwrap(), retained);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 100);
    let detached = pair.actual.snapshot(handle).unwrap();
    assert_eq!(
        pair.step("detached count", |h| h
            .dispatch_one(handle, MODULE_ID, COUNT, 0)),
        Ok(8)
    );
    assert_eq!(
        pair.step("detached stamp denied", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 31)),
        Err(Fault::NotActive)
    );
    assert_eq!(
        pair.step("invalid map preserves detached bundle", |h| h
            .attach(handle, 2)),
        Err(Fault::Invalid)
    );
    pair.assert_unchanged(handle, &detached);
    pair.step("attach restores own derived effect", |h| {
        h.attach(handle, 1)
    })
    .unwrap();
    assert_eq!(pair.actual.state(handle, MODULE_ID).unwrap(), retained);
    assert_eq!(pair.actual.payload_identity(handle).unwrap(), address);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 140);

    let policy = pair.actual.state(handle, 2).unwrap();
    let encounter = pair.actual.state(handle, 1).unwrap();
    pair.step("reset retains lifetime count", |h| {
        h.reset(handle, MODULE_ID)
    })
    .unwrap();
    assert_eq!(
        pair.state(handle),
        ExpeditionState {
            resets: 1,
            accepted_total: 8,
            checkpoints: vec![],
        }
    );
    assert_eq!(pair.actual.state(handle, 2).unwrap(), policy);
    assert_eq!(pair.actual.state(handle, 1).unwrap(), encounter);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 100);
    pair.step("new route after reset", |h| {
        h.dispatch_one(handle, MODULE_ID, STAMP, 10)
    })
    .unwrap();
    pair.step("detach before reset", |h| h.detach(handle))
        .unwrap();
    pair.step("reset is valid while detached", |h| {
        h.reset(handle, MODULE_ID)
    })
    .unwrap();
    assert_eq!(
        pair.state(handle),
        ExpeditionState {
            resets: 2,
            accepted_total: 9,
            checkpoints: vec![],
        }
    );
    pair.step("attach cleared route", |h| h.attach(handle, 0))
        .unwrap();
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 100);

    pair.step("prepare per-entity removal", |h| {
        h.dispatch_one(handle, MODULE_ID, STAMP, 14)
    })
    .unwrap();
    pair.step("remove only third state", |h| {
        h.remove_module_state(handle, MODULE_ID)
    })
    .unwrap();
    assert_eq!(pair.actual.state(handle, MODULE_ID), Err(Fault::Missing));
    assert_eq!(pair.actual.state(handle, 2).unwrap(), policy);
    assert_eq!(pair.actual.state(handle, 1).unwrap(), encounter);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 100);
    pair.step("reinstall fresh state through normal lifecycle", |h| {
        h.add_module_state(handle, MODULE_ID)
    })
    .unwrap();
    assert_eq!(pair.state(handle), ExpeditionState::default());
    pair.step("stamp reinstalled state", |h| {
        h.dispatch_one(handle, MODULE_ID, STAMP, 12)
    })
    .unwrap();
    let old = handle;
    handle = pair
        .step("replace incarnation", |h| h.replace(old, 1))
        .unwrap();
    pair.handles.push(handle);
    assert_eq!(handle.guid, old.guid);
    assert!(handle.generation > old.generation);
    assert_eq!(pair.state(handle), ExpeditionState::default());
    assert_eq!(
        pair.step("old incarnation denied", |h| h
            .dispatch_one(old, MODULE_ID, STAMP, 1)),
        Err(Fault::Stale)
    );
    let forged = Handle {
        guid: handle.guid,
        generation: handle.generation + 1,
    };
    assert_eq!(
        pair.step("forged incarnation denied", |h| h
            .dispatch_one(forged, MODULE_ID, STAMP, 1)),
        Err(Fault::Stale)
    );

    pair.step("stamp second entity before retirement", |h| {
        h.dispatch_one(other, MODULE_ID, STAMP, 3)
    })
    .unwrap();
    pair.step("retire runs module cleanup", |h| h.retire(other))
        .unwrap();
    assert_eq!(pair.actual.state(other, MODULE_ID), Err(Fault::Stale));
    let respawn = pair
        .step("same GUID is a new state owner", |h| h.spawn(other.guid, 0))
        .unwrap();
    pair.handles.push(respawn);
    assert!(respawn.generation > other.generation);
    assert_eq!(pair.state(respawn), ExpeditionState::default());
    pair.step("stamp first live instance", |h| {
        h.dispatch_one(handle, MODULE_ID, STAMP, 17)
    })
    .unwrap();
    pair.step("stamp second live instance", |h| {
        h.dispatch_one(respawn, MODULE_ID, STAMP, 18)
    })
    .unwrap();
    pair.step("policy remains independent", |h| {
        h.dispatch_one(handle, 2, event::POLICY, 20)
    })
    .unwrap();
    let surviving = pair.actual.state(handle, 2).unwrap();
    pair.step("unload clears every third contribution", |h| {
        h.unload_module(MODULE_ID)
    })
    .unwrap();
    for entity in [handle, respawn] {
        assert_eq!(pair.actual.state(entity, MODULE_ID), Err(Fault::Missing));
        assert!(
            !pair
                .actual
                .observables(entity)
                .unwrap()
                .by_module
                .iter()
                .any(|(id, _)| *id == MODULE_ID)
        );
    }
    assert_eq!(pair.actual.state(handle, 2).unwrap(), surviving);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 100);
    assert_eq!(
        pair.step("unloaded executable unavailable", |h| h
            .dispatch_one(handle, MODULE_ID, COUNT, 0)),
        Err(Fault::Missing)
    );
    assert!(
        pair.executed >= 40,
        "must execute the real module through the complete scenario"
    );
    partial_effects_and_overflow(mode);
}

fn partial_effects_and_overflow(mode: Mode) {
    let mut pair = Pair::new(
        mode,
        Limits {
            calls: 4,
            ..Limits::default()
        },
    );
    // The factory still registers all three; this entity exercises sparse membership.
    let handle = pair
        .step("sparse limited entity", |h| {
            h.spawn_with_modules(7399, 0, &[MODULE_ID])
        })
        .unwrap();
    pair.handles.push(handle);
    assert_eq!(
        pair.step("state write survives action budget failure", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 7)),
        Err(Fault::Limit)
    );
    assert_eq!(pair.state(handle).accepted_total, 1);
    assert_eq!(pair.state(handle).checkpoints, [7]);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 0);
    let partial = pair.actual.snapshot(handle).unwrap();
    assert_eq!(
        pair.step("retry cannot count accepted stamp twice", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 7)),
        Ok(1)
    );
    pair.assert_unchanged(handle, &partial);
    pair.step("detach accepted partial route", |h| h.detach(handle))
        .unwrap();
    pair.step("attach reconciles contribution", |h| h.attach(handle, 1))
        .unwrap();
    assert_eq!(pair.state(handle).accepted_total, 1);
    assert_eq!(pair.actual.observables(handle).unwrap().contribution, 5);

    pair.step("versioned maximum history replay", |h| {
        let mut snapshot = h.snapshot(handle)?;
        let record = snapshot
            .modules
            .iter_mut()
            .find(|m| m.id == MODULE_ID)
            .unwrap();
        let mut state = ExpeditionState::decode(&record.bytes)?;
        state.accepted_total = u64::MAX;
        state.resets = u32::MAX;
        record.bytes = state.encode();
        h.replay(handle, &snapshot)
    })
    .unwrap();
    let before = pair.actual.snapshot(handle).unwrap();
    assert_eq!(
        pair.step("history overflow precedes mutation", |h| h
            .dispatch_one(handle, MODULE_ID, STAMP, 8)),
        Err(Fault::Overflow)
    );
    pair.assert_unchanged(handle, &before);
    // Direct RESET invocation isolates the module rule. Host reset has a separate
    // documented pre-callback fence/contribution effect, not transactional rollback.
    assert_eq!(
        pair.step("module reset overflow precedes mutation", |h| h
            .dispatch_one(handle, MODULE_ID, event::RESET, 0)),
        Err(Fault::Overflow)
    );
    pair.assert_unchanged(handle, &before);
}

#[test]
fn independent_module_native_lifecycle() {
    lifecycle(Mode::Native);
}

#[cfg(feature = "wasm")]
#[test]
fn independent_module_rust_wasm_lifecycle() {
    lifecycle(Mode::RustWasm);
}

#[cfg(feature = "wasm")]
#[test]
fn independent_module_c_wasm_lifecycle() {
    lifecycle(Mode::CWasm);
}

#[cfg(feature = "wasm")]
#[test]
fn independent_module_mixed_lifecycle() {
    lifecycle(Mode::Mixed);
}
