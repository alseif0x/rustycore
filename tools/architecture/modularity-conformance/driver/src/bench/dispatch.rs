use super::{Result, Rng, checked, cpu_ticks, ns, population_digest, quantiles, rss};
use crate::composition::{Mode, build};
use conformance_contract::{Fault, event};
use conformance_encounter::STALE_OUTER_WRITE;
use conformance_host::Trace;
use conformance_policy::MODULE_ID as POLICY_ID;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, hint::black_box, time::Instant};

const WARMUP: usize = 256;

pub fn run(mode: Mode, workload: &str, calls: usize, seed: u64) -> Result<Value> {
    if !(1..=100_000).contains(&calls) || !matches!(workload, "policy" | "reentry") {
        return Err("dispatch requires 1..100000 calls and policy|reentry workload".into());
    }
    let construction = Instant::now();
    let mut host = checked(build(mode))?;
    let construct_ns = ns(construction);
    let handle = checked(host.spawn(42, 0))?;
    let mut rng = Rng(seed);
    let mut durations = Vec::with_capacity(calls);
    let mut expected_rejections = 0usize;
    let mut result_digest = Sha256::new();
    let mut event_counts = BTreeMap::<String, u64>::new();
    let mut admitted_calls = 0usize;
    let cpu_before = cpu_ticks();
    let mut invoke = |timed: bool| -> Result<()> {
        let argument = (rng.next() % 1_000_001) as i64;
        let event = if workload == "policy" {
            event::POLICY
        } else {
            STALE_OUTER_WRITE
        };
        let argument = if workload == "policy" { argument } else { 0 };
        let start = Instant::now();
        let result = black_box(host.dispatch(handle, event, argument));
        let elapsed = ns(start);
        if workload == "reentry" {
            if result != Err(Fault::Revision) {
                return Err(format!(
                    "expected stale outer CAS rejection after nested effect, got {result:?}"
                ));
            }
            if timed {
                expected_rejections += 1;
            }
        } else {
            let values = checked(result.clone())?;
            if values
                .iter()
                .find(|(module, _)| *module == POLICY_ID)
                .map(|(_, value)| *value)
                != Some(argument)
            {
                return Err(
                    "policy result disagrees with the independent default100% oracle".into(),
                );
            }
        }
        if timed {
            durations.push(elapsed);
            match result {
                Ok(values) => {
                    result_digest.update([0]);
                    result_digest.update((values.len() as u64).to_le_bytes());
                    for (module, value) in values {
                        result_digest.update(module.to_le_bytes());
                        result_digest.update(value.to_le_bytes());
                    }
                }
                Err(fault) => {
                    result_digest.update([1]);
                    result_digest.update(fault.code().to_le_bytes());
                }
            }
            admitted_calls += host.calls();
            for entry in host.trace() {
                if let Trace::Enter(frame) = entry {
                    *event_counts
                        .entry(format!("{}:{}", frame.module, frame.event))
                        .or_default() += 1;
                }
            }
        }
        Ok(())
    };
    for _ in 0..WARMUP {
        invoke(false)?;
    }
    let start = Instant::now();
    for _ in 0..calls {
        invoke(true)?;
    }
    let total_ns = ns(start);
    drop(invoke);
    let digest_start = Instant::now();
    let final_digest = population_digest(host.core(), &[handle])?;
    let materialize_ns = ns(digest_start);
    Ok(json!({
        "schema_version": 2, "kind": "dispatch-sample", "mode": mode.as_str(),
        "workload": workload, "calls": calls, "seed": seed, "warmup_calls": WARMUP,
        "construct_ns": construct_ns, "cold": host.cold_metrics(), "total_ns": total_ns,
        "latency": quantiles(&durations),
        "expected_rejections": expected_rejections, "final_digest": final_digest,
        "result_digest": format!("{:x}", result_digest.finalize()),
        "invocations_by_module_event": event_counts, "admitted_calls": admitted_calls,
        "materialize_ns": materialize_ns, "rss_kib": rss("VmRSS:"), "rss_hwm_kib": rss("VmHWM:"),
        "cpu_ticks_including_warmup_and_observation": cpu_before.zip(cpu_ticks()).map(|(a,b)| b.saturating_sub(a)),
        "timing_scope": "one root dispatch including all fanout/callbacks and budget reset; reentry workload intentionally rejects obsolete outer CAS after applying nested state",
        "total_scope": "measured loop also includes RNG, independent result validation, counters and result digest; not the sum of per-root latencies",
        "seed_usage": if workload == "policy" { "seeded varying arguments" } else { "fixed reentry input; seeds identify independent processes, not different semantic scenarios" },
        "reentry_boundary": "Encounter rejects obsolete outer CAS after nested fanout; later root callbacks do not run after that error",
        "raw_latencies_ns": durations,
    }))
}
