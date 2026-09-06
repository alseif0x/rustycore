//! Finite LAB assertions. catch_unwind reports failed assertions; it is NOT module isolation.
use super::{
    bench::{Config, Workload, observe},
    store::{Aggregate, Ecs},
    *,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CheckRow {
    pub backend: &'static str,
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}
#[derive(Debug, Serialize)]
pub struct CheckReport {
    pub schema_version: u8,
    pub mode: &'static str,
    pub lab_model: bool,
    pub ok: bool,
    pub checks: Vec<CheckRow>,
    pub limits: &'static str,
}

fn ordered<S: Store>(d: &Driver<S>) -> Vec<Row> {
    let mut rows = d.rows();
    rows.sort_unstable_by_key(|r| r.guid);
    rows
}

fn move_authority<S: Store>() {
    let mut d = Driver::<S>::default();
    let h = d.install(0, Bundle::new(11, true));
    d.write(h, |c| {
        c.x = 17;
        c.payload[0] = 99;
    })
    .unwrap();
    d.mutate_encounter(h, |s| s.timer = 1234).unwrap();
    let address = d.read(h, |c| c.payload.as_ptr() as usize).unwrap();
    d.detach(h).unwrap();
    assert_eq!(d.active(h), Err(Error::Detached));
    assert_eq!(d.maps.iter().map(Store::len).sum::<usize>(), 0);
    assert_eq!(d.detached.len(), 1);
    // Save-like detached reads/writes remain available; map admission does not.
    d.write(h, |c| c.revision = 73).unwrap();
    for (target, rejected) in [(1, true), (2, false)] {
        assert_eq!(d.attach(h, target, rejected), Err(Error::Rejected));
        assert_eq!(d.read(h, |c| c.payload.as_ptr() as usize).unwrap(), address);
        assert_eq!(d.detached.len(), 1);
        assert_eq!(d.owner(h).unwrap().map, None);
    }
    d.attach(h, 1, false).unwrap();
    assert!(d.detached.is_empty());
    assert_eq!(d.active(h).unwrap(), 1);
    assert_eq!(
        d.read(h, |c| (
            c.payload.as_ptr() as usize,
            c.payload[0],
            c.x,
            c.revision
        ))
        .unwrap(),
        (address, 99, 17, 73)
    );
    assert_eq!(d.encounter(h).unwrap().timer, 1234);
    assert_eq!(d.maps[1].policy(h.guid).unwrap().module, 2);
    assert_eq!(d.attach(h, 0, false), Err(Error::WrongMap));
    d.retire(h).unwrap();
    let replacement = d.install(0, Bundle::new(11, false));
    assert_ne!(replacement.generation, h.generation);
    assert_eq!(d.write(h, |c| c.health = 0), Err(Error::Stale));
    assert_eq!(d.detach(h), Err(Error::Stale));
    assert_eq!(d.read(replacement, |c| c.health).unwrap(), 750);
}

fn reciprocal_admission<S: Store>() {
    let mut d = Driver::<S>::default();
    let a = d.install(0, Bundle::new(1, false));
    let b = d.install(0, Bundle::new(2, false));
    let c = d.install(1, Bundle::new(3, false));
    let dead = d.install(0, Bundle::new(4, false));
    d.write(dead, |c| c.health = 0).unwrap();
    d.attack(a, b).unwrap();
    assert_eq!(d.read(a, |c| c.victim).unwrap(), Some(b.guid));
    assert!(d.read(b, |c| c.attackers.contains(&a.guid)).unwrap());
    let before = ordered(&d);
    let trace_before = d.trace.clone();
    assert_eq!(d.attack(a, c), Err(Error::WrongMap));
    assert_eq!(d.attack(a, a), Err(Error::WrongMap));
    assert_eq!(d.attack(a, dead), Err(Error::Rejected));
    assert_eq!(
        d.attack(a, Handle { generation: 0, ..b }),
        Err(Error::Stale)
    );
    assert_eq!(ordered(&d), before);
    assert_eq!(d.trace, trace_before); // No failure publication or partial reciprocal update.
    d.detach(c).unwrap();
    assert_eq!(d.attack(a, c), Err(Error::Detached));
    d.attach(c, 0, false).unwrap();
    d.attack(a, c).unwrap();
    assert!(d.read(b, |c| c.attackers.is_empty()).unwrap());
    assert!(d.read(c, |c| c.attackers.contains(&a.guid)).unwrap());
    d.detach(c).unwrap();
    assert_eq!(d.read(a, |c| c.victim).unwrap(), None);
    assert!(d.read(c, |c| c.attackers.is_empty()).unwrap());
}

fn independent_composition<S: Store>() {
    let mut d = Driver::<S>::default();
    let h = d.install(0, Bundle::new(1, false));
    d.maps[0].advance();
    assert_eq!(d.read(h, |c| c.x).unwrap(), 1);
    d.maps[0].set_optional(h.guid, true);
    d.policy(
        h,
        Policy {
            module: 2,
            bonus: 3,
        },
    )
    .unwrap();
    let before = ordered(&d);
    assert_eq!(
        d.policy(
            h,
            Policy {
                module: 3,
                bonus: 99
            }
        ),
        Err(Error::Conflict)
    );
    assert_eq!(ordered(&d), before);
    d.maps[0].set_optional(h.guid, false);
    assert!(d.maps[0].policy(h.guid).is_some());
    assert!(d.maps[0].encounter(h.guid).is_none());
    d.maps[0].advance();
    assert_eq!(d.read(h, |c| c.x).unwrap(), 5);
    d.maps[0].set_optional(h.guid, true);
    d.maps[0].remove_policy(h.guid);
    for _ in 0..8 {
        d.maps[0].advance();
    }
    assert_eq!(d.encounter(h).unwrap().pulses, 8);
    assert_eq!(d.read(h, |c| c.x).unwrap(), 15); // 8 base advances + the independent pulse.
    assert!(d.maps[0].policy(h.guid).is_none());
    d.policy(
        h,
        Policy {
            module: 3,
            bonus: 2,
        },
    )
    .unwrap();
    assert_eq!(d.maps[0].policy(h.guid).unwrap().module, 3);
}

fn encounter_reentry<S: Store>() {
    let mut d = Driver::<S>::default();
    let boss = d.install(0, Bundle::new(1, true));
    let target = d.install(0, Bundle::new(2, false));
    assert_eq!(d.encounter_step(boss, 100, false).unwrap(), None);
    assert_eq!(d.encounter(boss).unwrap().timer, 5000);
    d.attack(boss, target).unwrap();
    d.write(boss, |c| c.health = 400).unwrap();
    assert_eq!(d.encounter_step(boss, 100, true).unwrap(), None);
    let failed = d.encounter(boss).unwrap();
    assert_eq!(
        (
            failed.phase,
            failed.shield,
            failed.summon,
            failed.callbacks,
            failed.timer
        ),
        (1, true, None, 0, 4900)
    );
    let shield = d.trace.iter().position(|e| e.kind == "shield").unwrap();
    let failure = d
        .trace
        .iter()
        .position(|e| e.kind == "summon_failed")
        .unwrap();
    assert!(shield < failure);
    assert_eq!(d.maps[0].len(), 2);
    d.reset(boss).unwrap();
    d.trace.clear();
    let child = d.encounter_step(boss, 100, false).unwrap().unwrap();
    let callback = d
        .trace
        .iter()
        .position(|e| e.kind == "summoned_callback")
        .unwrap();
    let returned = d
        .trace
        .iter()
        .position(|e| e.kind == "summon_return")
        .unwrap();
    let read = d
        .trace
        .iter()
        .position(|e| e.kind == "read_after_callback")
        .unwrap();
    assert!(callback < returned && returned < read);
    assert_eq!(d.trace[callback].value, 1);
    assert_eq!(d.trace[read].value, 1);
    assert_eq!(d.callback_depth, 0);
    assert_eq!(
        d.maps[0].policy(child.guid),
        Some(Policy {
            module: 7,
            bonus: 1
        })
    );
    assert!(
        d.trace
            .iter()
            .any(|e| e.kind == "callback_policy" && e.guid == child.guid && e.value == 1)
    );
    assert_eq!(
        d.read(child, |c| (c.revision, c.victim)).unwrap(),
        (2, Some(target.guid))
    );
    assert!(
        d.read(target, |c| c.attackers.contains(&child.guid))
            .unwrap()
    );
    let paused = d.encounter(boss).unwrap();
    assert_eq!(d.encounter_step(boss, 9000, false).unwrap(), None);
    assert_eq!(d.encounter(boss).unwrap(), paused);
    assert_eq!(d.summoned_callback(boss, child), Err(Error::Conflict));
    assert_eq!(d.callback_depth, 0);
    assert_eq!(d.encounter(boss).unwrap().callbacks, 2); // Confirmed prior action is retained.
    // Explicit LAB teardown. Full C++ summon death, aura cleanup and DB Reset are not modeled.
    d.retire(child).unwrap();
    assert_eq!(d.summoned_callback(boss, child), Err(Error::Stale));
    assert_eq!(d.callback_depth, 0);
    assert_eq!(d.encounter(boss).unwrap().callbacks, 3);
    d.callback_depth = 4;
    assert_eq!(d.summoned_callback(boss, target), Err(Error::Rejected));
    assert_eq!(d.callback_depth, 4);
    assert_eq!(d.encounter(boss).unwrap().callbacks, 3);
    d.callback_depth = 0;
    d.stop(boss).unwrap();
    d.reset(boss).unwrap();
    assert_eq!(d.encounter(boss).unwrap(), Encounter::default());
    assert!(d.read(target, |c| c.attackers.is_empty()).unwrap());
}

fn actual_phase_barrier<S: Store>() {
    let mut d = Driver::<S>::default();
    let a = d.install(0, Bundle::new(1, false));
    let b = d.install(0, Bundle::new(2, false));
    let removed = d.install(1, Bundle::new(3, false));
    d.pending_attacks.push((a, b));
    d.deferred.push(removed);
    d.frame(&[], false).unwrap();
    assert_eq!(
        d.read(a, |c| (c.victim, c.health, c.x)).unwrap(),
        (Some(b.guid), 751, 1)
    );
    let barrier = d.trace.iter().position(|e| e.kind == "barrier").unwrap();
    assert_eq!(
        d.trace[..barrier]
            .iter()
            .filter(|e| e.kind == "objects_done")
            .count(),
        2
    );
    assert_eq!(d.trace[barrier].value, 3);
    let callback = d
        .trace
        .iter()
        .position(|e| e.kind == "far_callback_before_remove")
        .unwrap();
    let removal = d.trace.iter().position(|e| e.kind == "removed").unwrap();
    assert!(barrier < callback && callback < removal);
    assert_eq!(d.trace[callback].value, 1);
    assert_eq!(d.read(removed, |_| ()), Err(Error::Stale));
    assert_eq!(d.maps[1].len(), 0);
}

fn evaluate(
    checks: &mut Vec<CheckRow>,
    backend: &'static str,
    name: &'static str,
    f: impl FnOnce() + std::panic::UnwindSafe,
) {
    let outcome = std::panic::catch_unwind(f);
    let detail = match &outcome {
        Ok(()) => "finite LAB assertion passed".into(),
        Err(error) => error
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).into()))
            .unwrap_or_else(|| "assertion panicked".into()),
    };
    checks.push(CheckRow {
        backend,
        name,
        passed: outcome.is_ok(),
        detail,
    });
}

fn backend<S: Store>(checks: &mut Vec<CheckRow>) {
    for (name, test) in [
        (
            "non_clone_move_failed_attach_stale_replacement",
            move_authority::<S> as fn(),
        ),
        (
            "reciprocal_combat_atomic_admission",
            reciprocal_admission::<S>,
        ),
        (
            "independent_optional_families_conflict",
            independent_composition::<S>,
        ),
        (
            "timer_partial_failure_synchronous_callback_read_reset",
            encounter_reentry::<S>,
        ),
        (
            "invoked_map_barrier_callback_before_removal",
            actual_phase_barrier::<S>,
        ),
    ] {
        evaluate(checks, S::NAME, name, test);
    }
}

fn equivalence(seed: u64, density: &str) {
    let config = Config {
        backend: "aggregate".into(),
        entities: 32,
        ticks: 16,
        seed,
        density: density.into(),
    };
    let mut a = Workload::<Aggregate>::new(&config);
    let mut b = Workload::<Ecs>::new(&config);
    let (mut hash_a, mut hash_b) = (0xcbf29ce484222325, 0xcbf29ce484222325);
    for tick in 0..16 {
        let (mut ar, mut br) = (a.frame(tick), b.frame(tick));
        ar.sort_unstable_by_key(|r| r.guid);
        br.sort_unstable_by_key(|r| r.guid);
        assert_eq!(ar, br);
        assert_eq!(a.driver.trace, b.driver.trace); // Do not sort observable action order.
        observe(&mut hash_a, &ar, &a);
        observe(&mut hash_b, &br, &b);
        assert_eq!(hash_a, hash_b);
        a.driver.trace.clear();
        b.driver.trace.clear();
        for kind in 0..3 {
            let (ai, ak) = a.next_churn();
            let (bi, bk) = b.next_churn();
            assert_eq!((ai, ak), (bi, bk));
            // Every finite case covers all kinds, not just whichever RNG happened to select.
            a.churn(ai, kind);
            b.churn(bi, kind);
            assert_eq!(a.handles, b.handles);
            assert_eq!(ordered(&a.driver), ordered(&b.driver));
        }
    }
    let ar = ordered(&a.driver);
    let br = ordered(&b.driver);
    assert_eq!(ar.len(), 32);
    assert_eq!(
        ar.iter().filter(|r| r.encounter.is_some()).count(),
        if density == "dense" { 32 } else { 8 }
    );
    observe(&mut hash_a, &ar, &a);
    observe(&mut hash_b, &br, &b);
    assert_eq!(hash_a, hash_b);
}

pub fn check() -> CheckReport {
    let mut checks = Vec::new();
    backend::<Aggregate>(&mut checks);
    backend::<Ecs>(&mut checks);
    for seed in [0, 42, u64::MAX] {
        for density in ["sparse", "dense"] {
            evaluate(
                &mut checks,
                "cross-backend",
                if density == "sparse" {
                    "seeded_sparse_exact_trace_and_state"
                } else {
                    "seeded_dense_exact_trace_and_state"
                },
                || equivalence(seed, density),
            );
            checks
                .last_mut()
                .unwrap()
                .detail
                .push_str(&format!("; seed={seed}; density={density}"));
        }
    }
    CheckReport {
        schema_version: 1,
        mode: "check",
        lab_model: true,
        ok: checks.iter().all(|c| c.passed),
        checks,
        limits: "controlled synchronous two-map model; Bundle/Row/Store hard-code two optional types and ECS extraction enumerates four combinations; no independent arbitrary-type module SDK or maintenance-winner proof; no full production AI/physics, parallel workers, network, DB durability, module ABI, crash isolation or capture parity proof",
    }
}
