use super::{Result, Rng, checked, cpu_ticks, ns, population_digest, quantiles, rss};
use crate::composition::{Harness, Mode, build};
use conformance_contract::{Handle, event};
use serde_json::{Value, json};
use std::{hint::black_box, time::Instant};

const WARMUP: usize = 25;

fn frame(host: &mut Harness, handles: &[Handle]) -> Result<()> {
    for &handle in handles {
        // Visit every base entity, including those with zero optional state.
        black_box(checked(host.dispatch(handle, event::UPDATE, 0))?);
    }
    Ok(())
}

pub fn run(mode: Mode, population: usize, density: &str, ticks: usize, seed: u64) -> Result<Value> {
    if !(100..=10_000).contains(&population)
        || !(1..=200).contains(&ticks)
        || !matches!(density, "sparse" | "dense")
    {
        return Err("storage requires population100..10000, ticks1..200, sparse|dense".into());
    }
    let construction = Instant::now();
    let mut host = checked(build(mode))?;
    let construct_ns = ns(construction);
    let modules: Vec<_> = host.registered().iter().map(|m| m.id).collect();
    if modules.is_empty() {
        return Err("populated storage workload needs optional modules".into());
    }
    let optional = if density == "dense" {
        population
    } else {
        population / 4
    };
    let mut rng = Rng(seed);
    let mut guids: Vec<_> = (1..=population as u64).collect();
    for index in (1..guids.len()).rev() {
        let selected = (rng.next() % (index + 1) as u64) as usize;
        guids.swap(index, selected);
    }
    let create_start = Instant::now();
    let mut handles = Vec::with_capacity(population);
    for (index, guid) in guids.into_iter().enumerate() {
        let selected = if index < optional {
            modules.as_slice()
        } else {
            &[]
        };
        handles.push(checked(host.spawn_with_modules(
            guid,
            (index % 2) as u8,
            selected,
        ))?);
    }
    let create_ns = ns(create_start);
    // Shuffle visitation independently of insertion/membership: sparse is not a dense
    // prefix followed by an empty suffix in the measured traversal.
    for index in (1..handles.len()).rev() {
        let selected = (rng.next() % (index + 1) as u64) as usize;
        handles.swap(index, selected);
    }
    let optional_handles: Vec<_> = handles
        .iter()
        .copied()
        .filter(|&handle| !host.entity_modules(handle).expect("live handle").is_empty())
        .collect();
    let rss_after_population_kib = rss("VmRSS:");
    let cpu_before = cpu_ticks();
    for _ in 0..WARMUP {
        frame(&mut host, &handles)?;
    }
    let mut update = Vec::with_capacity(ticks);
    let mut churn = Vec::new();
    let mut transfer = Vec::new();
    let changed = population / 100;
    for tick in 0..ticks {
        let start = Instant::now();
        frame(&mut host, &handles)?;
        update.push(ns(start));
        // Membership churn and residence transfer are separate timed work, never hidden in update.
        for offset in 0..changed {
            let index = (tick * changed + offset) % optional;
            let module = modules[(tick + offset) % modules.len()];
            let start = Instant::now();
            checked(host.remove_module_state(optional_handles[index], module))?;
            checked(host.add_module_state(optional_handles[index], module))?;
            churn.push(ns(start));
        }
        for _ in 0..changed {
            let index = (rng.next() % population as u64) as usize;
            let current = match checked(host.residence(handles[index]))? {
                conformance_host::Residence::Active(map) => map,
                _ => return Err("transfer must start active".into()),
            };
            let target = 1 - current;
            let start = Instant::now();
            checked(host.detach(handles[index]))?;
            checked(host.attach(handles[index], target))?;
            transfer.push(ns(start));
        }
    }
    let materialize = Instant::now();
    let final_digest = population_digest(host.core(), &handles)?;
    let materialize_ns = ns(materialize);
    let final_optional = handles
        .iter()
        .filter(|&&handle| !host.entity_modules(handle).expect("live handle").is_empty())
        .count();
    let final_module_states: usize = handles
        .iter()
        .map(|&handle| host.entity_modules(handle).expect("live handle").len())
        .sum();
    let churn_total: u64 = churn.iter().sum();
    let transfer_total: u64 = transfer.iter().sum();
    Ok(json!({
        "schema_version": 2, "kind": "storage-sample", "mode": mode.as_str(),
        "population": population, "density": density, "ticks": ticks, "seed": seed,
        "warmup_ticks": WARMUP, "module_count": modules.len(), "optional_entities": optional,
        "final_entities": handles.len(), "final_optional_entities": final_optional,
        "final_module_states": final_module_states, "final_digest": final_digest,
        "construct_ns": construct_ns, "cold": host.cold_metrics(), "create_ns": create_ns,
        "materialize_ns": materialize_ns,
        "update": quantiles(&update), "churn": quantiles(&churn), "transfer": quantiles(&transfer),
        "churn_total_ns": churn_total, "transfer_total_ns": transfer_total,
        "operations": { "updates": population * ticks, "churn": changed * ticks,
            "transfer": changed * ticks, "cross_map_transfers": changed * ticks, "same_map_transfers": 0 },
        "rss_after_population_kib": rss_after_population_kib,
        "rss_kib": rss("VmRSS:"), "rss_hwm_kib": rss("VmHWM:"),
        "cpu_ticks_including_warmup_and_observation": cpu_before.zip(cpu_ticks()).map(|(a,b)| b.saturating_sub(a)),
        "unit": "one churn=remove+reinstall one optional state; one transfer=active->detached->active with lifecycle; update p99 is a whole-population batch",
        "workload_boundary": "per-handle dispatcher/lookup/codecs, not a pure hecs query; after warmup most UPDATE calls read phase1, while churn reintroduces phase0 writes; sparse visitation is shuffled independently of insertion",
        "raw_update_ns": update, "raw_churn_ns": churn, "raw_transfer_ns": transfer,
    }))
}
